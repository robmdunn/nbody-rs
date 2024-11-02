#[derive(Clone, Debug)]
pub struct Body {
    pub mass: f64,
    pub position: [f64; 2],  // [x, y]
    pub velocity: [f64; 2],  // [vx, vy]
    pub acceleration: [f64; 2],  // [ax, ay]
}

impl Body {
    pub fn new(mass: f64, x: f64, y: f64, vx: f64, vy: f64) -> Self {
        Body {
            mass,
            position: [x, y],
            velocity: [vx, vy],
            acceleration: [0.0, 0.0],
        }
    }

    pub fn update_position(&mut self, dt: f64) {
        // Update position based on velocity
        self.position[0] += self.velocity[0] * dt;
        self.position[1] += self.velocity[1] * dt;
    }

    pub fn update_velocity(&mut self, dt: f64) {
        // Update velocity based on acceleration
        self.velocity[0] += self.acceleration[0] * dt;
        self.velocity[1] += self.acceleration[1] * dt;
    }
}