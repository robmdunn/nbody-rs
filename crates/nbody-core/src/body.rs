// Generic body structure supporting both 2D and 3D
#[derive(Clone, Debug)]
pub struct Body<const N: usize> {
    pub mass: f64,
    pub position: [f64; N],
    pub velocity: [f64; N],
    pub acceleration: [f64; N],
}

impl<const N: usize> Body<N> {
    pub fn new_with_arrays(mass: f64, position: [f64; N], velocity: [f64; N]) -> Self {
        Body {
            mass,
            position,
            velocity,
            acceleration: [0.0; N],
        }
    }

    pub fn update_position(&mut self, dt: f64) {
        for i in 0..N {
            self.position[i] += self.velocity[i] * dt;
        }
    }

    pub fn update_velocity(&mut self, dt: f64) {
        for i in 0..N {
            self.velocity[i] += self.acceleration[i] * dt;
        }
    }
}

// 2D specific implementation for backward compatibility
impl Body<2> {
    pub fn new(mass: f64, x: f64, y: f64, vx: f64, vy: f64) -> Self {
        Body {
            mass,
            position: [x, y],
            velocity: [vx, vy],
            acceleration: [0.0, 0.0],
        }
    }
}

// 3D specific implementation
impl Body<3> {
    pub fn new_3d(mass: f64, x: f64, y: f64, z: f64, vx: f64, vy: f64, vz: f64) -> Self {
        Body {
            mass,
            position: [x, y, z],
            velocity: [vx, vy, vz],
            acceleration: [0.0, 0.0, 0.0],
        }
    }
}

// Type aliases for convenience and backward compatibility
pub type Body2D = Body<2>;
pub type Body3D = Body<3>;

// Re-export the generic Body directly for backward compatibility
// The existing code will still work as Body<2> is the same as the old Body struct