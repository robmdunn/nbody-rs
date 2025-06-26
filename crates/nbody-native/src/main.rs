// crates/nbody-native/src/main.rs
use clap::Parser;
use rand::prelude::*;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::num::NonZeroU32;
use winit::{
    event::{Event, WindowEvent, MouseButton, ElementState, MouseScrollDelta},
    event_loop::{ControlFlow, EventLoop},
    window::{WindowBuilder, Window},
    dpi::{LogicalSize, PhysicalPosition},
};
use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextAttributesBuilder, PossiblyCurrentContext},
    display::GetGlDisplay,
    prelude::*,
    surface::{Surface, SwapInterval, WindowSurface},
};
use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::HasRawWindowHandle;
use std::sync::Arc;

mod fileio;

use nbody_core::{Body2D as Body, Body3D, Simulation, Simulation3D, Renderer, Renderer3D};

const DEFAULT_BODIES: usize = 1000;
const DEFAULT_MASS: f64 = 2000.0;
const DEFAULT_G: f64 = 6.67384e-11;
const DEFAULT_TIMESTEP: f64 = 0.1;
const DEFAULT_SOFTENING: f64 = 0.005;
const DEFAULT_SPIN: f64 = 0.05;
const DEFAULT_MZERO: f64 = 1.0e7;
const DEFAULT_TREE_RATIO: f64 = 3.0;
const DEFAULT_WRITE_INTERVAL: usize = 100;
const FRAME_TIME: Duration = Duration::from_micros(66666); // Approximately 30 FPS
const PI: f64 = std::f32::consts::PI as f64;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Config {
    /// Number of bodies to simulate
    #[arg(short = 'n', long, default_value_t = DEFAULT_BODIES)]
    n_bodies: usize,

    /// Mass for randomly distributed bodies
    #[arg(short = 'm', long, default_value_t = DEFAULT_MASS)]
    mass: f64,

    /// Gravitational constant
    #[arg(short = 'g', long, default_value_t = DEFAULT_G)]
    g: f64,

    /// Simulation timestep
    #[arg(short = 'd', long = "dt", default_value_t = DEFAULT_TIMESTEP)]
    timestep: f64,

    /// Softening factor to prevent singularities
    #[arg(short = 'f', long = "sf", default_value_t = DEFAULT_SOFTENING)]
    softening: f64,

    /// Initial spin factor for random distribution
    #[arg(short = 's', long, default_value_t = DEFAULT_SPIN)]
    spin: f64,

    /// Mass of central body
    #[arg(long = "mz", default_value_t = DEFAULT_MZERO)]
    mzero: f64,

    /// Tree ratio threshold for Barnes-Hut approximation
    #[arg(short = 't', long = "tr", default_value_t = DEFAULT_TREE_RATIO)]
    tree_ratio: f64,

    /// Input file to resume simulation from
    #[arg(short = 'r', long = "resume")]
    input_file: Option<PathBuf>,

    /// Output file to save simulation state
    #[arg(short = 'o', long = "output")]
    output_file: Option<PathBuf>,

    /// Interval (in steps) between writing output
    #[arg(long = "nsteps", default_value_t = DEFAULT_WRITE_INTERVAL)]
    write_interval: usize,

    /// Disable graphics
    #[arg(long = "no-graphics")]
    no_graphics: bool,

    /// Window width
    #[arg(long, default_value_t = 800)]
    width: u32,

    /// Window height
    #[arg(long, default_value_t = 800)]
    height: u32,

    /// Point size for rendering bodies
    #[arg(short = 'p', long, default_value_t = 2.0)]
    point_size: f32,

    /// Use fixed scale view instead of following particles
    #[arg(long)]
    fixed_scale: bool,

    /// Enable 3D simulation mode
    #[arg(long)]
    mode_3d: bool,

    /// Show wireframe in 3D mode
    #[arg(long)]
    wireframe: bool,
}

enum SimulationMode {
    Mode2D {
        simulation: Simulation,
        renderer: Option<Renderer>,
    },
    Mode3D {
        simulation: Simulation3D,
        renderer: Option<Renderer3D>,
    },
}

struct SimulationState {
    mode: SimulationMode,
    gl_context: Option<PossiblyCurrentContext>,
    gl_surface: Option<Surface<WindowSurface>>,
    step_count: usize,
    sim_time: f64,
    last_render: Instant,
    last_save: usize,
    frame_times: Vec<Duration>,  // Track recent frame times
    fps_update_timer: Instant,   // Timer for FPS updates
    // Camera controls for 3D mode
    mouse_pressed: bool,
    last_mouse_pos: PhysicalPosition<f64>,
    camera_theta: f32,  // Horizontal rotation around Y axis
    camera_phi: f32,    // Vertical rotation
    camera_distance: f32,
}

impl SimulationState {
    fn new_2d(simulation: Simulation) -> Self {
        SimulationState {
            mode: SimulationMode::Mode2D {
                simulation,
                renderer: None,
            },
            gl_context: None,
            gl_surface: None,
            step_count: 0,
            sim_time: 0.0,
            last_render: Instant::now(),
            last_save: 0,
            frame_times: Vec::with_capacity(60),
            fps_update_timer: Instant::now(),
            // Camera controls (unused in 2D mode)
            mouse_pressed: false,
            last_mouse_pos: PhysicalPosition::new(0.0, 0.0),
            camera_theta: 0.0,
            camera_phi: 0.0,
            camera_distance: 10.0,
        }
    }

    fn new_3d(simulation: Simulation3D) -> Self {
        SimulationState {
            mode: SimulationMode::Mode3D {
                simulation,
                renderer: None,
            },
            gl_context: None,
            gl_surface: None,
            step_count: 0,
            sim_time: 0.0,
            last_render: Instant::now(),
            last_save: 0,
            frame_times: Vec::with_capacity(60),
            fps_update_timer: Instant::now(),
            // Camera controls for 3D mode - start at a better viewing angle
            mouse_pressed: false,
            last_mouse_pos: PhysicalPosition::new(0.0, 0.0),
            camera_theta: std::f32::consts::PI * 0.25,    // 45 degrees around Y axis
            camera_phi: std::f32::consts::PI * 0.15,      // 15 degrees up from horizon
            camera_distance: 5.0, // Closer to the action
        }
    }

    fn init_renderer(
        &mut self,
        event_loop: &EventLoop<()>,
        config: &Config,
    ) -> Result<Window, Box<dyn std::error::Error>> {
        let window_builder = WindowBuilder::new()
            .with_title("N-body Simulation")
            .with_inner_size(LogicalSize::new(
                config.width as f64,
                config.height as f64,
            ));

        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_transparency(true);

        let display_builder = DisplayBuilder::new().with_window_builder(Some(window_builder));
        let (window, gl_config) = display_builder
            .build(event_loop, template, |configs| {
                configs
                    .reduce(|accum, config| {
                        let transparency_check = config.supports_transparency().unwrap_or(false)
                            & !accum.supports_transparency().unwrap_or(false);
                        if transparency_check || config.num_samples() > accum.num_samples() {
                            config
                        } else {
                            accum
                        }
                    })
                    .unwrap()
            })
            .map_err(|e| format!("Failed to build window: {}", e))?;

        let window = window.unwrap();
        let raw_window_handle = window.raw_window_handle();

        let gl_display = gl_config.display();
        let context_attributes = ContextAttributesBuilder::new().build(Some(raw_window_handle));
        let gl_context = unsafe {
            gl_display
                .create_context(&gl_config, &context_attributes)
                .map_err(|e| format!("Failed to create context: {}", e))?
        };

        let attrs = window.build_surface_attributes(<_>::default());
        let gl_surface = unsafe {
            gl_display
                .create_window_surface(&gl_config, &attrs)
                .map_err(|e| format!("Failed to create surface: {}", e))?
        };

        let gl_context = gl_context
            .make_current(&gl_surface)
            .map_err(|e| format!("Failed to make context current: {}", e))?;

        // Create glow context
        let gl = unsafe {
            let gl = glow::Context::from_loader_function(|s| {
                gl_display.get_proc_address(&std::ffi::CString::new(s).unwrap()) as *const _
            });
            Arc::new(gl)
        };

        gl_surface
            .set_swap_interval(&gl_context, SwapInterval::Wait(NonZeroU32::new(1).unwrap()))
            .map_err(|e| format!("Failed to set swap interval: {}", e))?;

        // Initialize renderer based on mode
        match &mut self.mode {
            SimulationMode::Mode2D { renderer, .. } => {
                let mut renderer_2d = Renderer::new(gl, config.point_size, config.fixed_scale)?;
                renderer_2d.set_wireframe(config.wireframe);
                *renderer = Some(renderer_2d);
            }
            SimulationMode::Mode3D { renderer, .. } => {
                let aspect_ratio = config.width as f32 / config.height as f32;
                let mut renderer_3d = Renderer3D::new(gl, config.point_size, aspect_ratio)?;
                renderer_3d.set_wireframe(config.wireframe);
                *renderer = Some(renderer_3d);
                // Set initial camera position for 3D mode
                self.update_camera_3d();
            }
        }
        self.gl_context = Some(gl_context);
        self.gl_surface = Some(gl_surface);

        Ok(window)
    }

    fn update(&mut self, config: &Config) -> Result<(), String> {
        // Step simulation based on mode
        match &mut self.mode {
            SimulationMode::Mode2D { simulation, .. } => {
                simulation.step();
            }
            SimulationMode::Mode3D { simulation, .. } => {
                simulation.step();
            }
        }
        
        self.step_count += 1;
        self.sim_time += config.timestep;

        // Save state if requested (only for 2D mode for now)
        if let Some(ref output_file) = config.output_file {
            if self.step_count % config.write_interval == 0 {
                if let SimulationMode::Mode2D { simulation, .. } = &self.mode {
                    fileio::write_bodies(
                        output_file,
                        simulation.bodies(),
                        config.timestep,
                        config.g,
                        config.softening,
                        config.tree_ratio,
                    )?;
                    self.last_save = self.step_count;
                }
            }
        }

        // Update FPS counter every second
        if self.fps_update_timer.elapsed() >= Duration::from_secs(1) {
            if !self.frame_times.is_empty() {
                let avg_frame_time = self.frame_times.iter().sum::<Duration>() / self.frame_times.len() as u32;
                let fps = 1.0 / avg_frame_time.as_secs_f64();
                print!("\r{} {:<12.6} seconds | {:.1} FPS", 
                    console::style("Simulation time:").cyan(),
                    self.sim_time,
                    fps
                );
            }
            self.fps_update_timer = Instant::now();
            self.frame_times.clear();
        }

        Ok(())
    }

    fn should_render(&self) -> bool {
        self.last_render.elapsed() >= FRAME_TIME
    }

    fn handle_mouse_input(&mut self, button: MouseButton, state: ElementState, position: PhysicalPosition<f64>) {
        match button {
            MouseButton::Left => {
                self.mouse_pressed = state == ElementState::Pressed;
                if self.mouse_pressed {
                    self.last_mouse_pos = position;
                }
            }
            _ => {}
        }
    }

    fn handle_mouse_motion(&mut self, position: PhysicalPosition<f64>) {
        if self.mouse_pressed {
            let dx = (position.x - self.last_mouse_pos.x) as f32;
            let dy = (position.y - self.last_mouse_pos.y) as f32;
            
            // Rotate camera based on mouse movement
            let sensitivity = 0.01;
            self.camera_theta += dx * sensitivity;
            self.camera_phi += dy * sensitivity; // Reverse Y for different feel
            
            // Clamp phi to prevent gimbal lock
            self.camera_phi = self.camera_phi.clamp(-std::f32::consts::PI * 0.48, std::f32::consts::PI * 0.48);
            
            self.last_mouse_pos = position;
            
            // Update camera position
            self.update_camera_3d();
        }
    }

    fn handle_scroll(&mut self, delta_y: f32) {
        // Zoom in/out with scroll wheel
        let zoom_speed = 0.5;
        self.camera_distance = (self.camera_distance - delta_y * zoom_speed).clamp(2.0, 50.0);
        self.update_camera_3d();
    }

    fn update_camera_3d(&mut self) {
        if let SimulationMode::Mode3D { renderer, .. } = &mut self.mode {
            if let Some(renderer) = renderer {
                let camera = renderer.camera_mut();
                
                // Convert spherical coordinates to cartesian
                // theta=0 should be +Z axis, phi=0 should be XZ plane
                let x = self.camera_distance * self.camera_phi.cos() * self.camera_theta.sin();
                let y = self.camera_distance * self.camera_phi.sin();
                let z = self.camera_distance * self.camera_phi.cos() * self.camera_theta.cos();
                
                camera.position = [x, y, z];
                camera.target = [0.0, 0.0, 0.0]; // Always look at origin
            }
        }
    }

    fn render(&mut self) {
        if let (Some(gl_surface), Some(gl_context)) = 
            (self.gl_surface.as_ref(), self.gl_context.as_ref()) {
            let frame_start = Instant::now();
            
            match &self.mode {
                SimulationMode::Mode2D { simulation, renderer } => {
                    if let Some(renderer) = renderer {
                        let tree = simulation.get_tree();
                        renderer.render(simulation.bodies(), &tree);
                    }
                }
                SimulationMode::Mode3D { simulation, renderer } => {
                    if let Some(renderer) = renderer {
                        let tree = simulation.get_tree();
                        renderer.render(simulation.bodies(), &tree);
                    }
                }
            }
            
            gl_surface.swap_buffers(gl_context).unwrap();
            
            // Track frame time
            self.frame_times.push(frame_start.elapsed());
            self.last_render = Instant::now();
        }
    }
}

fn random_bodies(config: &Config) -> Vec<Body> {
    let mut rng = rand::thread_rng();
    let mut bodies = Vec::with_capacity(config.n_bodies);

    // Create central body first
    bodies.push(Body::new(
        config.mzero,
        0.0, 0.0,  // position
        0.0, 0.0   // velocity
    ));

    // Create remaining bodies
    for _ in 1..config.n_bodies {
        let r = rng.gen::<f64>() * 2.0 - 1.0; // Range [-1, 1]
        let theta = 2.0 * PI * rng.gen::<f64>();

        let x = r * theta.cos();
        let y = r * theta.sin();

        let mut vx = 0.0;
        let mut vy = 0.0;

        if config.spin != 0.0 {
            let spin_factor = config.spin * (1.0 + 0.1 * rng.gen::<f64>()) / (1.0 + r.abs());
            vx = -y * spin_factor; // Tangential velocity
            vy = x * spin_factor;
        }

        bodies.push(Body::new(config.mass, x, y, vx, vy));
    }

    bodies
}

fn random_bodies_3d(config: &Config) -> Vec<Body3D> {
    let mut rng = rand::thread_rng();
    let mut bodies = Vec::with_capacity(config.n_bodies);

    // Create central body first
    let central_body = Body3D::new_3d(
        config.mzero,
        0.0, 0.0, 0.0,  // position
        0.0, 0.0, 0.0   // velocity
    );
    println!("3D Central body: mass={}, pos=[{}, {}, {}]", 
        central_body.mass, central_body.position[0], central_body.position[1], central_body.position[2]);
    bodies.push(central_body);

    // Create remaining bodies in 3D space
    for i in 1..config.n_bodies {
        // Generate random spherical coordinates with larger scale
        let r = rng.gen::<f64>() * 10.0 - 5.0; // Range [-5, 5] - much larger scale
        let theta = 2.0 * PI * rng.gen::<f64>(); // Azimuthal angle
        let phi = PI * rng.gen::<f64>(); // Polar angle

        let x = r * phi.sin() * theta.cos();
        let y = r * phi.sin() * theta.sin();
        let z = r * phi.cos();

        // Debug first few bodies
        if i <= 3 {
            println!("3D Body {}: r={:.2}, x={:.2}, y={:.2}, z={:.2}", i, r, x, y, z);
        }

        let mut vx = 0.0;
        let mut vy = 0.0;
        let mut vz = 0.0;

        if config.spin != 0.0 {
            let spin_factor = config.spin * (1.0 + 0.1 * rng.gen::<f64>()) / (1.0 + r.abs());
            // Create orbital motion around z-axis (like a disk)
            vx = -y * spin_factor; 
            vy = x * spin_factor;
            vz = 0.0; // Keep motion primarily in xy plane initially
        }

        bodies.push(Body3D::new_3d(config.mass, x, y, z, vx, vy, vz));
    }

    bodies
}

fn run_simulation(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    // Create simulation state based on mode
    let mut state = if config.mode_3d {
        // 3D mode
        let bodies = random_bodies_3d(&config);
        let simulation = Simulation3D::new(
            bodies,
            config.timestep,
            config.g,
            config.softening,
            config.tree_ratio
        );
        SimulationState::new_3d(simulation)
    } else {
        // 2D mode (default)
        let bodies = if let Some(ref input_file) = config.input_file {
            fileio::read_bodies(input_file)?
        } else {
            random_bodies(&config)
        };
        let simulation = Simulation::new(
            bodies,
            config.timestep,
            config.g,
            config.softening,
            config.tree_ratio
        );
        SimulationState::new_2d(simulation)
    };

    // Print initial configuration
    println!("{}",
        console::style("N-body Simulation Configuration")
            .bold()
            .bright()
            .underlined()
    );
    println!("{}: {}", 
        console::style("Number of bodies").cyan(),
        console::style(config.n_bodies).yellow()
    );
    println!("{}: {}", 
        console::style("Timestep").cyan(),
        console::style(config.timestep).yellow()
    );
    println!("{}: {}", 
        console::style("Graphics").cyan(),
        console::style(!config.no_graphics).yellow()
    );
    println!("{}: {}", 
        console::style("Mode").cyan(),
        console::style(if config.mode_3d { "3D" } else { "2D" }).yellow()
    );

    if !config.no_graphics {
        let event_loop = EventLoop::new();
        let window = state.init_renderer(&event_loop, &config)?;

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    *control_flow = ControlFlow::Exit;
                }
                Event::WindowEvent {
                    event: WindowEvent::MouseInput { button, state: element_state, .. },
                    ..
                } => {
                    if config.mode_3d {
                        state.handle_mouse_input(button, element_state, state.last_mouse_pos);
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::CursorMoved { position, .. },
                    ..
                } => {
                    if config.mode_3d {
                        state.handle_mouse_motion(position);
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::MouseWheel { delta, .. },
                    ..
                } => {
                    if config.mode_3d {
                        let delta_y = match delta {
                            MouseScrollDelta::LineDelta(_, y) => y,
                            MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.01,
                        };
                        state.handle_scroll(delta_y);
                    }
                }
                Event::MainEventsCleared => {
                    if let Err(e) = state.update(&config) {
                        eprintln!("Error updating simulation: {}", e);
                        *control_flow = ControlFlow::Exit;
                        return;
                    }

                    if state.should_render() {
                        state.render();
                    }
                }
                _ => (),
            }
        });
    } else {
        // Non-graphical simulation loop
        loop {
            if let Err(e) = state.update(&config) {
                eprintln!("Error updating simulation: {}", e);
                break;
            }
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::parse();
    run_simulation(config)
}