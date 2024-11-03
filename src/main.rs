use clap::Parser;
use rand::prelude::*;
use std::path::PathBuf;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
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

fn run_simulation(mut config: Config) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize bodies either from file or random distribution
    let bodies = if let Some(ref input_file) = config.input_file {
        read_bodies(input_file)?
    } else {
        random_bodies(&config)
    };

    // Create simulation
    let mut simulation = Simulation::new(
        bodies,
        config.timestep,
        config.g,
        config.softening,
        config.tree_ratio
    );

    // Print initial configuration using colorful output
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

    let mut step_count = 0;
    let mut sim_time = 0.0;

    if !config.no_graphics {
        let event_loop = EventLoop::new();
        
        // Initialize renderer first
        render::init_window(&event_loop, config.width, config.height, config.point_size)?;

        // Run event loop
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            match event {
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                    *control_flow = ControlFlow::Exit;
                }
                Event::MainEventsCleared => {
                    // Update simulation
                    simulation.step();
                    step_count += 1;
                    sim_time += config.timestep;

                    // Save state if requested
                    if let Some(ref output_file) = config.output_file {
                        if step_count % config.write_interval == 0 {
                            if let Err(e) = write_bodies(
                                output_file,
                                simulation.bodies(),
                                config.timestep,
                                config.g,
                                config.softening,
                                config.tree_ratio,
                            ) {
                                eprintln!("Error writing to file: {}", e);
                            }
                        }
                    }

                    // Update status
                    print!("\r{} {:<12.6} seconds", 
                        console::style("Simulation time:").cyan(),
                        sim_time
                    );

                    // Render
                    let tree = simulation.get_tree();
                    render::draw(simulation.bodies(), &tree);
                }
                _ => (),
            }
        });
    } else {
        // Non-graphical simulation loop
        loop {
            simulation.step();
            step_count += 1;
            sim_time += config.timestep;

            if let Some(ref output_file) = config.output_file {
                if step_count % config.write_interval == 0 {
                    write_bodies(
                        output_file,
                        simulation.bodies(),
                        config.timestep,
                        config.g,
                        config.softening,
                        config.tree_ratio,
                    )?;
                }
            }

            print!("\r{} {:<12.6} seconds", 
                console::style("Simulation time:").cyan(),
                sim_time
            );
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let config = Config::parse();

    // Run simulation
    run_simulation(config)
}