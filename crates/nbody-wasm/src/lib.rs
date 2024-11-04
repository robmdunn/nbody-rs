use wasm_bindgen::prelude::*;
use web_sys::{WebGlRenderingContext, HtmlCanvasElement};
use nbody_core::{Simulation, Body, Renderer};
use std::sync::Arc;

#[wasm_bindgen]
pub struct NBodySimulation {
    simulation: Simulation,
    renderer: Renderer,
    gl_context: WebGlRenderingContext,
}

#[wasm_bindgen]
impl NBodySimulation {
    #[wasm_bindgen(constructor)]
    pub fn new(
        canvas: HtmlCanvasElement,
        n_bodies: usize,
        point_size: f32,
        fixed_scale: bool
    ) -> Result<NBodySimulation, JsValue> {
        // Set up WebGL context
        let gl_context = canvas
            .get_context("webgl2")?
            .unwrap()
            .dyn_into::<WebGlRenderingContext>()?;

        // Create glow context
        let gl = Arc::new(glow::Context::from_webgl2_context(gl_context.clone()));
        
        // Initialize renderer
        let renderer = Renderer::new(gl, point_size, fixed_scale)
            .map_err(|e| JsValue::from_str(&e))?;

        // Initialize simulation with random bodies
        let simulation = Simulation::new(
            create_random_bodies(n_bodies),
            0.1,    // timestep
            6.67384e-11, // G
            0.005,  // softening
            3.0,    // tree_threshold
        );

        Ok(NBodySimulation {
            simulation,
            renderer,
            gl_context,
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

fn create_random_bodies(n_bodies: usize) -> Vec<Body> {
    // Implementation similar to the original random_bodies function
    // but adapted for WASM context...
}

// Required by wasm-bindgen
#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();
    Ok(())
}