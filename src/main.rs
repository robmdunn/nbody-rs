use clap::Parser;
use rand::prelude::*;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop}
};

mod body;
mod fileio;
mod render;
mod simulation;
mod tree;

use crate::body::Body;
use crate::simulation::Simulation;
use crate::fileio::{read_bodies, write_bodies};

const DEFAULT_BODIES: usize = 1000;
const DEFAULT_MASS: f64 = 2000.0;
const DEFAULT_G: f64 = 6.67384e-11;
const DEFAULT_TIMESTEP: f64 = 0.1;
const DEFAULT_SOFTENING: f64 = 0.005;
const DEFAULT_SPIN: f64 = 0.05;
const DEFAULT_MZERO: f64 = 1.0e7;
const DEFAULT_TREE_RATIO: f64 = 3.0;
const DEFAULT_WRITE_INTERVAL: usize = 100;
const FRAME_TIME: Duration = Duration::from_micros(66666); // Approximately 1/30th second
const PI: f64 = std::f32::consts::PI as f64;

/// Barnes-Hut N-body gravitational simulation
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

struct SimulationState {
    simulation: Simulation,
    step_count: usize,
    sim_time: f64,
    last_render: Instant,
    last_save: usize,
}

impl SimulationState {
    fn new(simulation: Simulation) -> Self {
        SimulationState {
            simulation,
            step_count: 0,
            sim_time: 0.0,
            last_render: Instant::now(),
            last_save: 0,
        }
    }

    fn update(&mut self, config: &Config) -> Result<(), String> {
        self.simulation.step();
        self.step_count += 1;
        self.sim_time += config.timestep;

        // Save state if requested
        if let Some(ref output_file) = config.output_file {
            if self.step_count % config.write_interval == 0 {
                write_bodies(
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

        // Update status line
        print!("\r{} {:<12.6} seconds", 
            console::style("Simulation time:").cyan(),
            self.sim_time
        );

        Ok(())
    }

    fn should_render(&self) -> bool {
        self.last_render.elapsed() >= FRAME_TIME
    }

    fn render(&mut self) {
        if render::window_open() {
            let tree = self.simulation.get_tree();
            render::draw(self.simulation.bodies(), &tree);
            self.last_render = Instant::now();
        }
    }
}

fn run_simulation(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize bodies either from file or random distribution
    let bodies = if let Some(ref input_file) = config.input_file {
        read_bodies(input_file)?
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
        render::init_window(&event_loop, config.width, config.height, config.point_size)?;

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            match event {
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                    *control_flow = ControlFlow::Exit;
                }
                Event::MainEventsCleared => {
                    // Always update simulation
                    if let Err(e) = state.update(&config) {
                        eprintln!("Error updating simulation: {}", e);
                        *control_flow = ControlFlow::Exit;
                        return;
                    }

                    // Only render if enough time has passed
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