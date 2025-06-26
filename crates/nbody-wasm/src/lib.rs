use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext, HtmlCanvasElement};
use nbody_core::{Simulation, Simulation3D, Body2D as Body, Body3D, Renderer, Renderer3D};
use std::sync::Arc;
use std::f64::consts::PI;
use rand::Rng;

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
    pub mode_3d: bool,
    pub show_wireframe: bool,
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
            mode_3d: false,
            show_wireframe: true,
        }
    }
}

enum SimulationMode {
    Mode2D {
        simulation: Simulation,
        renderer: Renderer,
    },
    Mode3D {
        simulation: Simulation3D,
        renderer: Renderer3D,
    },
}

#[wasm_bindgen]
pub struct NBodySimulation {
    mode: SimulationMode,
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

        // Create glow context using WebGL2
        let gl = Arc::new(glow::Context::from_webgl2_context(gl_context));

        let mode = if config.mode_3d {
            // 3D mode
            let aspect_ratio = canvas.width() as f32 / canvas.height() as f32;
            let mut renderer = Renderer3D::new(gl, config.point_size, aspect_ratio)
                .map_err(|e| JsValue::from_str(&e))?;
            renderer.set_wireframe(config.show_wireframe);
            let simulation = Simulation3D::new(
                create_random_bodies_3d(config),
                config.timestep,
                config.g,
                config.softening,
                config.tree_ratio,
            );
            SimulationMode::Mode3D { simulation, renderer }
        } else {
            // 2D mode
            let mut renderer = Renderer::new(gl, config.point_size, config.fixed_scale)
                .map_err(|e| JsValue::from_str(&e))?;
            renderer.set_wireframe(config.show_wireframe);
            let simulation = Simulation::new(
                create_random_bodies(config),
                config.timestep,
                config.g,
                config.softening,
                config.tree_ratio,
            );
            SimulationMode::Mode2D { simulation, renderer }
        };

        Ok(NBodySimulation { mode })
    }

    pub fn step(&mut self) {
        match &mut self.mode {
            SimulationMode::Mode2D { simulation, .. } => {
                simulation.step();
            }
            SimulationMode::Mode3D { simulation, .. } => {
                simulation.step();
            }
        }
    }

    pub fn render(&self) {
        match &self.mode {
            SimulationMode::Mode2D { simulation, renderer } => {
                let bodies = simulation.bodies();
                let tree = simulation.get_tree();
                renderer.render(bodies, &tree);
            }
            SimulationMode::Mode3D { simulation, renderer } => {
                let bodies = simulation.bodies();
                let tree = simulation.get_tree();
                renderer.render(bodies, &tree);
            }
        }
    }

    // Mouse controls for 3D mode
    pub fn handle_mouse_down(&mut self, x: f32, y: f32) {
        // Store mouse position for 3D camera controls
        // JavaScript will handle the mouse state tracking
    }

    pub fn handle_mouse_move(&mut self, dx: f32, dy: f32) {
        if let SimulationMode::Mode3D { renderer, .. } = &mut self.mode {
            let camera = renderer.camera_mut();
            
            // Simple rotation based on mouse movement
            let sensitivity = 0.01;
            
            // Calculate new camera position based on mouse movement
            // This is a simplified version - you might want to add proper spherical coordinates
            let current_pos = camera.position;
            let distance = (current_pos[0] * current_pos[0] + current_pos[1] * current_pos[1] + current_pos[2] * current_pos[2]).sqrt();
            
            // Simple rotation around Y and X axes
            let theta = dx * sensitivity;
            let phi = -dy * sensitivity; // Reverse Y for different feel
            
            // Apply rotation (simplified)
            let cos_theta = theta.cos();
            let sin_theta = theta.sin();
            let cos_phi = phi.cos();
            let sin_phi = phi.sin();
            
            // Rotate around Y axis (theta)
            let new_x = current_pos[0] * cos_theta - current_pos[2] * sin_theta;
            let new_z = current_pos[0] * sin_theta + current_pos[2] * cos_theta;
            
            // Rotate around X axis (phi) 
            let new_y = current_pos[1] * cos_phi - new_z * sin_phi;
            let final_z = current_pos[1] * sin_phi + new_z * cos_phi;
            
            camera.position = [new_x, new_y, final_z];
        }
    }

    pub fn handle_scroll(&mut self, delta_y: f32) {
        if let SimulationMode::Mode3D { renderer, .. } = &mut self.mode {
            let camera = renderer.camera_mut();
            
            // Zoom in/out by changing distance
            let zoom_speed = 0.1;
            let current_pos = camera.position;
            let distance = (current_pos[0] * current_pos[0] + current_pos[1] * current_pos[1] + current_pos[2] * current_pos[2]).sqrt();
            let new_distance = (distance + delta_y * zoom_speed).clamp(2.0, 50.0);
            
            // Scale position to new distance
            let scale = new_distance / distance;
            camera.position = [
                current_pos[0] * scale,
                current_pos[1] * scale,
                current_pos[2] * scale,
            ];
        }
    }

    pub fn set_wireframe(&mut self, show_wireframe: bool) {
        match &mut self.mode {
            SimulationMode::Mode2D { renderer, .. } => {
                renderer.set_wireframe(show_wireframe);
            }
            SimulationMode::Mode3D { renderer, .. } => {
                renderer.set_wireframe(show_wireframe);
            }
        }
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

fn create_random_bodies_3d(config: &SimConfig) -> Vec<Body3D> {
    let mut rng = rand::thread_rng();
    let mut bodies = Vec::with_capacity(config.n_bodies);

    // Create central body first
    bodies.push(Body3D::new_3d(
        config.mzero,
        0.0, 0.0, 0.0,  // position
        0.0, 0.0, 0.0   // velocity
    ));

    // Create remaining bodies in 3D space
    for _ in 1..config.n_bodies {
        // Generate random spherical coordinates
        let r = rng.gen::<f64>() * 10.0 - 5.0; // Range [-5, 5]
        let theta = 2.0 * PI * rng.gen::<f64>(); // Azimuthal angle
        let phi = PI * rng.gen::<f64>(); // Polar angle

        let x = r * phi.sin() * theta.cos();
        let y = r * phi.sin() * theta.sin();
        let z = r * phi.cos();

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

// Required by wasm-bindgen
#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    Ok(())
}