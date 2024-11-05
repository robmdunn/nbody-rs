// crates/nbody-native/src/main.rs
use clap::Parser;
use rand::prelude::*;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::num::NonZeroU32;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{WindowBuilder, Window},
    dpi::LogicalSize,
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

use nbody_core::{Body, Simulation, Renderer};

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
}

struct SimulationState {
    simulation: Simulation,
    renderer: Option<Renderer>,
    gl_context: Option<PossiblyCurrentContext>,
    gl_surface: Option<Surface<WindowSurface>>,
    step_count: usize,
    sim_time: f64,
    last_render: Instant,
    last_save: usize,
    frame_times: Vec<Duration>,  // Track recent frame times
    fps_update_timer: Instant,   // Timer for FPS updates
}

impl SimulationState {
    fn new(simulation: Simulation) -> Self {
        SimulationState {
            simulation,
            renderer: None,
            gl_context: None,
            gl_surface: None,
            step_count: 0,
            sim_time: 0.0,
            last_render: Instant::now(),
            last_save: 0,
            frame_times: Vec::with_capacity(60),
            fps_update_timer: Instant::now(),
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

        // Initialize renderer
        self.renderer = Some(Renderer::new(gl, config.point_size, config.fixed_scale)?);
        self.gl_context = Some(gl_context);
        self.gl_surface = Some(gl_surface);

        Ok(window)
    }

    fn update(&mut self, config: &Config) -> Result<(), String> {
        self.simulation.step();
        self.step_count += 1;
        self.sim_time += config.timestep;

        // Save state if requested
        if let Some(ref output_file) = config.output_file {
            if self.step_count % config.write_interval == 0 {
                fileio::write_bodies(
                    output_file,
                    self.simulation.bodies(),
                    config.timestep,
                    config.g,
                    config.softening,
                    config.tree_ratio,
                )?;
                self.last_save = self.step_count;
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

    fn render(&mut self) {
        if let (Some(renderer), Some(gl_surface), Some(gl_context)) = 
            (self.renderer.as_ref(), self.gl_surface.as_ref(), self.gl_context.as_ref()) {
            let frame_start = Instant::now();
            
            let tree = self.simulation.get_tree();
            renderer.render(self.simulation.bodies(), &tree);
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

fn run_simulation(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize bodies either from file or random distribution
    let bodies = if let Some(ref input_file) = config.input_file {
        fileio::read_bodies(input_file)?
    } else {
        random_bodies(&config)
    };

    // Create simulation
    let simulation = Simulation::new(
        bodies,
        config.timestep,
        config.g,
        config.softening,
        config.tree_ratio
    );

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

    let mut state = SimulationState::new(simulation);

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