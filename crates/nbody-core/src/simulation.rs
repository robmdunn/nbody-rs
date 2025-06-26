#[cfg(feature = "parallel")]
use rayon::prelude::*;
use crate::body::{Body2D as Body};
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
    fn compute_bounds(&self) -> Bounds {
        if self.bodies.is_empty() {
            return Bounds::new([-1.0, -1.0], [1.0, 1.0]); // Default bounds for empty system
        }

        // Start with the first body's position
        let first_pos = self.bodies[0].position;
        let mut min_x = first_pos[0];
        let mut min_y = first_pos[1];
        let mut max_x = first_pos[0];
        let mut max_y = first_pos[1];

        // Find the actual extents of all bodies
        for body in &self.bodies[1..] {
            min_x = min_x.min(body.position[0]);
            min_y = min_y.min(body.position[1]);
            max_x = max_x.max(body.position[0]);
            max_y = max_y.max(body.position[1]);
        }

        // Handle the case where all bodies are at exactly the same point
        if (max_x - min_x).abs() < f64::EPSILON {
            max_x += f64::EPSILON;
            min_x -= f64::EPSILON;
        }
        if (max_y - min_y).abs() < f64::EPSILON {
            max_y += f64::EPSILON;
            min_y -= f64::EPSILON;
        }

        Bounds::new([min_x, min_y], [max_x, max_y])
    }

    /// Build the quad tree from the current body positions
    fn build_tree(&self) -> QuadTree {
        let bounds = self.compute_bounds();
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
        let g = self.g;
        let softening = self.softening;
        let threshold = self.tree_threshold;

        // Calculate forces/accelerations using parallel or sequential iteration
        #[cfg(feature = "parallel")]
        {
            self.bodies.par_iter_mut().for_each(|body| {
                // Reset acceleration
                body.acceleration = [0.0, 0.0];
                
                // Calculate force
                let force = tree.calculate_force(
                    body,
                    g,
                    softening,
                    threshold
                );

                // Update acceleration (F = ma -> a = F/m)
                body.acceleration = [
                    force[0] / body.mass,
                    force[1] / body.mass
                ];
            });
        }

        #[cfg(not(feature = "parallel"))]
        {
            self.bodies.iter_mut().for_each(|body| {
                // Reset acceleration
                body.acceleration = [0.0, 0.0];
                
                // Calculate force
                let force = tree.calculate_force(
                    body,
                    g,
                    softening,
                    threshold
                );

                // Update acceleration (F = ma -> a = F/m)
                body.acceleration = [
                    force[0] / body.mass,
                    force[1] / body.mass
                ];
            });
        }
    }

    /// Update velocities based on current accelerations
    fn update_velocities(&mut self) {
        let dt = self.timestep;
        
        #[cfg(feature = "parallel")]
        {
            self.bodies.par_iter_mut().for_each(|body| {
                body.update_velocity(dt);
            });
        }

        #[cfg(not(feature = "parallel"))]
        {
            self.bodies.iter_mut().for_each(|body| {
                body.update_velocity(dt);
            });
        }
    }

    /// Update positions based on current velocities
    fn update_positions(&mut self) {
        let dt = self.timestep;
        
        #[cfg(feature = "parallel")]
        {
            self.bodies.par_iter_mut().for_each(|body| {
                body.update_position(dt);
            });
        }

        #[cfg(not(feature = "parallel"))]
        {
            self.bodies.iter_mut().for_each(|body| {
                body.update_position(dt);
            });
        }
    }

    /// Perform one simulation step
    pub fn step(&mut self) {
        // Calculate new accelerations
        self.calculate_accelerations();

        // Update velocities and positions
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
    fn test_bounds_growth() {
        // Create two bodies moving outward
        let bodies = vec![
            Body::new(1.0, -1.0, -1.0, -1.0, -1.0),  // Moving left and down
            Body::new(1.0, 1.0, 1.0, 1.0, 1.0),      // Moving right and up
        ];
        let mut sim = Simulation::new(bodies, 0.1, 0.0, 0.001, 0.5);  // Set g=0 to prevent attraction

        // Get initial bounds
        let initial_bounds = sim.get_tree().get_bounds().clone();

        // Step simulation several times
        for _ in 0..10 {
            sim.step();
        }

        // Get new bounds
        let binding = sim.get_tree();
        let new_bounds = binding.get_bounds();

        // Verify bounds have grown
        assert!(new_bounds.min[0] < initial_bounds.min[0]);
        assert!(new_bounds.min[1] < initial_bounds.min[1]);
        assert!(new_bounds.max[0] > initial_bounds.max[0]);
        assert!(new_bounds.max[1] > initial_bounds.max[1]);
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

// 3D SIMULATION ENGINE

use crate::body::Body3D;
use crate::tree::{OctTree, Bounds3D};

pub struct Simulation3D {
    bodies: Vec<Body3D>,
    timestep: f64,
    g: f64,
    softening: f64,
    tree_threshold: f64,
}

impl Simulation3D {
    pub fn new(bodies: Vec<Body3D>, timestep: f64, g: f64, softening: f64, tree_threshold: f64) -> Self {
        Simulation3D {
            bodies,
            timestep,
            g,
            softening,
            tree_threshold,
        }
    }

    /// Get a reference to the current bodies in the simulation
    pub fn bodies(&self) -> &[Body3D] {
        &self.bodies
    }

    /// Calculate the boundaries that contain all bodies in 3D space
    fn compute_bounds(&self) -> Bounds3D {
        if self.bodies.is_empty() {
            return Bounds3D::new([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]); // Default bounds for empty system
        }

        // Start with the first body's position
        let first_pos = self.bodies[0].position;
        let mut min_x = first_pos[0];
        let mut min_y = first_pos[1];
        let mut min_z = first_pos[2];
        let mut max_x = first_pos[0];
        let mut max_y = first_pos[1];
        let mut max_z = first_pos[2];

        // Find the actual extents of all bodies
        for body in &self.bodies[1..] {
            min_x = min_x.min(body.position[0]);
            min_y = min_y.min(body.position[1]);
            min_z = min_z.min(body.position[2]);
            max_x = max_x.max(body.position[0]);
            max_y = max_y.max(body.position[1]);
            max_z = max_z.max(body.position[2]);
        }

        // Handle the case where all bodies are at exactly the same point
        if (max_x - min_x).abs() < f64::EPSILON {
            max_x += f64::EPSILON;
            min_x -= f64::EPSILON;
        }
        if (max_y - min_y).abs() < f64::EPSILON {
            max_y += f64::EPSILON;
            min_y -= f64::EPSILON;
        }
        if (max_z - min_z).abs() < f64::EPSILON {
            max_z += f64::EPSILON;
            min_z -= f64::EPSILON;
        }

        Bounds3D::new([min_x, min_y, min_z], [max_x, max_y, max_z])
    }

    /// Build the octree from the current body positions
    fn build_tree(&self) -> OctTree {
        let bounds = self.compute_bounds();
        let mut tree = OctTree::new(bounds);

        // Insert all bodies into the tree
        for body in &self.bodies {
            tree.insert(body.clone());
        }

        tree
    }

    /// Calculate accelerations for all bodies using the Barnes-Hut algorithm in 3D
    fn calculate_accelerations(&mut self) {
        // Build the octree
        let tree = self.build_tree();
        let g = self.g;
        let softening = self.softening;
        let threshold = self.tree_threshold;

        // Calculate forces/accelerations using parallel or sequential iteration
        #[cfg(feature = "parallel")]
        {
            self.bodies.par_iter_mut().for_each(|body| {
                // Reset acceleration
                body.acceleration = [0.0, 0.0, 0.0];
                
                // Calculate force
                let force = tree.calculate_force(
                    body,
                    g,
                    softening,
                    threshold
                );

                // Update acceleration (F = ma -> a = F/m)
                body.acceleration = [
                    force[0] / body.mass,
                    force[1] / body.mass,
                    force[2] / body.mass
                ];
            });
        }

        #[cfg(not(feature = "parallel"))]
        {
            self.bodies.iter_mut().for_each(|body| {
                // Reset acceleration
                body.acceleration = [0.0, 0.0, 0.0];
                
                // Calculate force
                let force = tree.calculate_force(
                    body,
                    g,
                    softening,
                    threshold
                );

                // Update acceleration (F = ma -> a = F/m)
                body.acceleration = [
                    force[0] / body.mass,
                    force[1] / body.mass,
                    force[2] / body.mass
                ];
            });
        }
    }

    /// Update velocities based on current accelerations
    fn update_velocities(&mut self) {
        let dt = self.timestep;
        
        #[cfg(feature = "parallel")]
        {
            self.bodies.par_iter_mut().for_each(|body| {
                body.update_velocity(dt);
            });
        }

        #[cfg(not(feature = "parallel"))]
        {
            self.bodies.iter_mut().for_each(|body| {
                body.update_velocity(dt);
            });
        }
    }

    /// Update positions based on current velocities
    fn update_positions(&mut self) {
        let dt = self.timestep;
        
        #[cfg(feature = "parallel")]
        {
            self.bodies.par_iter_mut().for_each(|body| {
                body.update_position(dt);
            });
        }

        #[cfg(not(feature = "parallel"))]
        {
            self.bodies.iter_mut().for_each(|body| {
                body.update_position(dt);
            });
        }
    }

    /// Perform one simulation step
    pub fn step(&mut self) {
        // Calculate new accelerations
        self.calculate_accelerations();

        // Update velocities and positions
        self.update_velocities();
        self.update_positions();
    }

    /// Get a reference to the octree for visualization purposes
    pub fn get_tree(&self) -> OctTree {
        self.build_tree()
    }
}

#[cfg(test)]
mod tests_3d {
    use super::*;

    #[test]
    fn test_simulation_3d_creation() {
        let bodies = vec![
            Body3D::new_3d(1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            Body3D::new_3d(1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0),
        ];
        let sim = Simulation3D::new(bodies, 0.1, 1.0, 0.001, 0.5);
        assert_eq!(sim.bodies.len(), 2);
    }

    #[test]
    fn test_3d_simulation_step() {
        // Create two bodies that should attract each other in 3D
        let bodies = vec![
            Body3D::new_3d(1.0, -0.5, 0.0, 0.0, 0.0, 0.0, 0.0),
            Body3D::new_3d(1.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0),
        ];

        let mut sim = Simulation3D::new(bodies, 0.1, 1.0, 0.001, 0.5);

        // Store initial positions
        let initial_x1 = sim.bodies[0].position[0];
        let initial_x2 = sim.bodies[1].position[0];

        // Run one step
        sim.step();

        // Bodies should move towards each other
        assert!(sim.bodies[0].position[0] > initial_x1);
        assert!(sim.bodies[1].position[0] < initial_x2);
    }

    #[test]
    fn test_3d_bounds_computation() {
        // Create bodies in 3D space
        let bodies = vec![
            Body3D::new_3d(1.0, -1.0, -1.0, -1.0, 0.0, 0.0, 0.0),
            Body3D::new_3d(1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0),
        ];

        let sim = Simulation3D::new(bodies, 0.1, 1.0, 0.001, 0.5);
        let bounds = sim.compute_bounds();

        assert_eq!(bounds.min, [-1.0, -1.0, -1.0]);
        assert_eq!(bounds.max, [1.0, 1.0, 1.0]);
    }
}