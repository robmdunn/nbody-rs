use rayon::prelude::*;
use crate::body::Body;
use crate::tree::{QuadTree, Bounds};

pub struct Simulation {
    bodies: Vec<Body>,
    timestep: f64,
    g: f64,
    softening: f64,
    tree_threshold: f64,
}

impl Simulation {
    pub fn new(bodies: Vec<Body>, timestep: f64, g: f64, softening: f64, tree_threshold: f64) -> Self {
        Simulation {
            bodies,
            timestep,
            g,
            softening,
            tree_threshold,
        }
    }

    /// Get a reference to the current bodies in the simulation
    pub fn bodies(&self) -> &[Body] {
        &self.bodies
    }

    /// Calculate the boundaries that contain all bodies
    fn calculate_bounds(&self) -> Bounds {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for body in &self.bodies {
            min_x = min_x.min(body.position[0]);
            min_y = min_y.min(body.position[1]);
            max_x = max_x.max(body.position[0]);
            max_y = max_y.max(body.position[1]);
        }

        // Add some padding to ensure bodies at the edges are handled correctly
        let padding = ((max_x - min_x) + (max_y - min_y)).max(1e-6) * 0.01;
        Bounds::new(
            [min_x - padding, min_y - padding],
            [max_x + padding, max_y + padding]
        )
    }

    /// Build the quad tree from the current body positions
    fn build_tree(&self) -> QuadTree {
        let bounds = self.calculate_bounds();
        let mut tree = QuadTree::new(bounds);
        
        // Insert all bodies into the tree
        for body in &self.bodies {
            tree.insert(body.clone());
        }
        
        tree
    }

    /// Calculate accelerations for all bodies using the Barnes-Hut algorithm
    fn calculate_accelerations(&mut self) {
        // Build the quad tree
        let tree = self.build_tree();

        // Calculate forces/accelerations in parallel
        self.bodies.par_iter_mut().for_each(|body| {
            let force = tree.calculate_force(
                body,
                self.g,
                self.softening,
                self.tree_threshold
            );
            
            // F = ma -> a = F/m
            body.acceleration = [
                force[0] / body.mass,
                force[1] / body.mass
            ];
        });
    }

    /// Update velocities based on current accelerations
    fn update_velocities(&mut self) {
        self.bodies.par_iter_mut().for_each(|body| {
            body.update_velocity(self.timestep);
        });
    }

    /// Update positions based on current velocities
    fn update_positions(&mut self) {
        self.bodies.par_iter_mut().for_each(|body| {
            body.update_position(self.timestep);
        });
    }

    /// Perform one simulation step
    pub fn step(&mut self) {
        // Calculate new accelerations
        self.calculate_accelerations();
        
        // Update velocities and positions
        // Note: in a more sophisticated implementation, we might want to use
        // a better integration method like leap-frog or RK4
        self.update_velocities();
        self.update_positions();
    }

    /// Get a reference to the quad tree for visualization purposes
    pub fn get_tree(&self) -> QuadTree {
        self.build_tree()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulation_creation() {
        let bodies = vec![
            Body::new(1.0, 0.0, 0.0, 0.0, 0.0),
            Body::new(1.0, 1.0, 0.0, 0.0, 0.0),
        ];
        let sim = Simulation::new(bodies, 0.1, 1.0, 0.001, 0.5);
        assert_eq!(sim.bodies.len(), 2);
    }

    #[test]
    fn test_bounds_calculation() {
        let bodies = vec![
            Body::new(1.0, -1.0, -1.0, 0.0, 0.0),
            Body::new(1.0, 1.0, 1.0, 0.0, 0.0),
        ];
        let sim = Simulation::new(bodies, 0.1, 1.0, 0.001, 0.5);
        let bounds = sim.calculate_bounds();
        
        // Check bounds with padding
        assert!(bounds.min[0] < -1.0);
        assert!(bounds.min[1] < -1.0);
        assert!(bounds.max[0] > 1.0);
        assert!(bounds.max[1] > 1.0);
    }

    #[test]
    fn test_simulation_step() {
        // Create two bodies that should attract each other
        let bodies = vec![
            Body::new(1.0, -0.5, 0.0, 0.0, 0.0),
            Body::new(1.0, 0.5, 0.0, 0.0, 0.0),
        ];
        
        let mut sim = Simulation::new(bodies, 0.1, 1.0, 0.001, 0.5);
        
        // Store initial positions
        let initial_x1 = sim.bodies[0].position[0];
        let initial_x2 = sim.bodies[1].position[0];
        
        // Run one step
        sim.step();
        
        // Bodies should move towards each other
        assert!(sim.bodies[0].position[0] > initial_x1);
        assert!(sim.bodies[1].position[0] < initial_x2);
    }
}