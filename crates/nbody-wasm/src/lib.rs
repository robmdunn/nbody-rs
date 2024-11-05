use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext, HtmlCanvasElement};
use nbody_core::{Simulation, Body, Renderer};
use std::sync::Arc;
use std::f64::consts::PI;
use rand::Rng;
use glow::Context as GlowContext;

#[wasm_bindgen]
pub struct SimConfig {
    pub n_bodies: usize,
    pub mass: f64,
    pub g: f64,
    pub timestep: f64,
    pub softening: f64,
    pub spin: f64,
    pub mzero: f64,
    pub tree_ratio: f64,
    pub point_size: f32,
    pub fixed_scale: bool,
}

#[wasm_bindgen]
impl SimConfig {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        SimConfig {
            n_bodies: 1000,
            mass: 2000.0,
            g: 6.67384e-11,
            timestep: 0.1,
            softening: 0.005,
            spin: 0.05,
            mzero: 1.0e7,
            tree_ratio: 3.0,
            point_size: 2.0,
            fixed_scale: false,
        }
    }
}

#[wasm_bindgen]
pub struct NBodySimulation {
    simulation: Simulation,
    renderer: Renderer,
}

#[wasm_bindgen]
impl NBodySimulation {
    #[wasm_bindgen(constructor)]
    pub fn new(
        canvas: HtmlCanvasElement,
        config: &SimConfig,
    ) -> Result<NBodySimulation, JsValue> {
        // Set up panic hook for better error messages
        console_error_panic_hook::set_once();

        // Get WebGL2 context
        let gl_context = canvas
            .get_context("webgl2")?
            .ok_or_else(|| JsValue::from_str("Failed to get WebGL2 context"))?
            .dyn_into::<WebGl2RenderingContext>()?;

        // Create glow context
        let gl = unsafe {
            let gl = GlowContext::from_webgl2_context(gl_context);
            Arc::new(gl)
        };

        // Initialize renderer
        let renderer = Renderer::new(gl, config.point_size, config.fixed_scale)
            .map_err(|e| JsValue::from_str(&e))?;

        // Initialize simulation with random bodies
        let simulation = Simulation::new(
            create_random_bodies(config),
            config.timestep,
            config.g,
            config.softening,
            config.tree_ratio,
        );

        Ok(NBodySimulation {
            simulation,
            renderer,
        })
    }

    pub fn step(&mut self) {
        self.simulation.step();
    }

    pub fn render(&self) {
        let bodies = self.simulation.bodies();
        let tree = self.simulation.get_tree();
        self.renderer.render(bodies, &tree);
    }
}

fn create_random_bodies(config: &SimConfig) -> Vec<Body> {
    let mut rng = rand::thread_rng();
    let mut bodies = Vec::with_capacity(config.n_bodies);

    // Create central body
    bodies.push(Body::new(
        config.mzero,
        0.0, 0.0,  // position
        0.0, 0.0   // velocity
    ));

    // Create remaining bodies
    for _ in 1..config.n_bodies {
        let r = rng.gen::<f64>() * 2.0 - 1.0;
        let theta = 2.0 * PI * rng.gen::<f64>();

        let x = r * theta.cos();
        let y = r * theta.sin();

        // Add some initial velocity for orbit
        let spin_factor = config.spin * (1.0 + 0.1 * rng.gen::<f64>()) / (1.0 + r.abs());
        let vx = -y * spin_factor;
        let vy = x * spin_factor;

        bodies.push(Body::new(config.mass, x, y, vx, vy));
    }

    bodies
}

// Required by wasm-bindgen
#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    Ok(())
}