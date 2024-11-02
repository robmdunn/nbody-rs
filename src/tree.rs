use std::cmp::Ordering;
use crate::body::Body;

#[derive(Debug, Clone)]
pub struct Bounds {
    pub min: [f64; 2],
    pub max: [f64; 2],
}

impl Bounds {
    pub fn new(min: [f64; 2], max: [f64; 2]) -> Self {
        Bounds { min, max }
    }

    pub fn center(&self) -> [f64; 2] {
        [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
        ]
    }

    pub fn diagonal(&self) -> f64 {
        let dx = self.max[0] - self.min[0];
        let dy = self.max[1] - self.min[1];
        (dx * dx + dy * dy).sqrt()
    }

    pub fn contains(&self, point: [f64; 2]) -> bool {
        point[0] >= self.min[0] && point[0] <= self.max[0] &&
        point[1] >= self.min[1] && point[1] <= self.max[1]
    }

    fn subdivide(&self) -> [Bounds; 4] {
        let center = self.center();
        [
            // Quadrant 1 (top right)
            Bounds::new([center[0], center[1]], [self.max[0], self.max[1]]),
            // Quadrant 2 (top left)
            Bounds::new([self.min[0], center[1]], [center[0], self.max[1]]),
            // Quadrant 3 (bottom left)
            Bounds::new([self.min[0], self.min[1]], [center[0], center[1]]),
            // Quadrant 4 (bottom right)
            Bounds::new([center[0], self.min[1]], [center[0], center[1]]),
        ]
    }
}

#[derive(Debug)]
pub struct QuadTree {
    bounds: Bounds,
    total_mass: f64,
    center_of_mass: [f64; 2],
    body: Option<Box<Body>>,
    children: [Option<Box<QuadTree>>; 4],
}

impl QuadTree {
    pub fn new(bounds: Bounds) -> Self {
        QuadTree {
            bounds,
            total_mass: 0.0,
            center_of_mass: [0.0, 0.0],
            body: None,
            children: [None, None, None, None],
        }
    }

    pub fn insert(&mut self, mut body: Body) {
        // If this node is empty, store the body here
        if self.total_mass == 0.0 {
            self.total_mass = body.mass;
            self.center_of_mass = body.position;
            self.body = Some(Box::new(body));
            return;
        }

        // If this node already contains a body, split it
        if let Some(existing_body) = self.body.take() {
            self.subdivide_and_insert(*existing_body);
        }

        // Insert the new body into the appropriate quadrant
        self.subdivide_and_insert(body);

        // Update center of mass and total mass
        self.update_mass_distribution();
    }

    fn subdivide_and_insert(&mut self, body: Body) {
        let quadrant = self.get_quadrant(body.position);
        let child = &mut self.children[quadrant];

        if child.is_none() {
            let bounds = self.bounds.subdivide()[quadrant].clone();
            *child = Some(Box::new(QuadTree::new(bounds)));
        }

        if let Some(ref mut child) = child {
            child.insert(body);
        }
    }

    fn get_quadrant(&self, position: [f64; 2]) -> usize {
        let center = self.bounds.center();
        match (position[0].partial_cmp(&center[0]), position[1].partial_cmp(&center[1])) {
            (Some(Ordering::Greater), Some(Ordering::Greater)) => 0, // Quadrant 1
            (Some(Ordering::Less | Ordering::Equal), Some(Ordering::Greater)) => 1, // Quadrant 2
            (Some(Ordering::Less | Ordering::Equal), Some(Ordering::Less | Ordering::Equal)) => 2, // Quadrant 3
            (Some(Ordering::Greater), Some(Ordering::Less | Ordering::Equal)) => 3, // Quadrant 4
            _ => 0, // Handle NaN cases by defaulting to quadrant 1
        }
    }

    fn update_mass_distribution(&mut self) {
        let mut total_mass = 0.0;
        let mut com_x = 0.0;
        let mut com_y = 0.0;

        // Add contribution from direct body if present
        if let Some(ref body) = self.body {
            total_mass += body.mass;
            com_x += body.mass * body.position[0];
            com_y += body.mass * body.position[1];
        }

        // Add contributions from children
        for child in self.children.iter().flatten() {
            total_mass += child.total_mass;
            com_x += child.total_mass * child.center_of_mass[0];
            com_y += child.total_mass * child.center_of_mass[1];
        }

        if total_mass > 0.0 {
            self.center_of_mass = [com_x / total_mass, com_y / total_mass];
        }
        self.total_mass = total_mass;
    }

    pub fn calculate_force(&self, body: &Body, g: f64, softening: f64, threshold: f64) -> [f64; 2] {
        // Don't calculate force with self
        if let Some(ref node_body) = self.body {
            if std::ptr::eq(body, &**node_body) {
                return [0.0, 0.0];
            }
        }

        let dx = self.center_of_mass[0] - body.position[0];
        let dy = self.center_of_mass[1] - body.position[1];
        let distance_sq = dx * dx + dy * dy;
        let distance = distance_sq.sqrt();

        // If this is a leaf node or the node is sufficiently far away
        if self.is_leaf() || (self.bounds.diagonal() / distance) < threshold {
            if distance_sq == 0.0 {
                return [0.0, 0.0];
            }

            // Calculate gravitational force
            let force = (g * body.mass * self.total_mass) / (distance_sq + softening);
            let force_x = force * dx / distance;
            let force_y = force * dy / distance;
            
            return [force_x, force_y];
        }

        // Otherwise, recursively calculate forces from children
        let mut total_force = [0.0, 0.0];
        for child in self.children.iter().flatten() {
            let force = child.calculate_force(body, g, softening, threshold);
            total_force[0] += force[0];
            total_force[1] += force[1];
        }

        total_force
    }

    fn is_leaf(&self) -> bool {
        self.children.iter().all(|child| child.is_none())
    }

    // For visualization purposes
    pub fn get_bounds(&self) -> &Bounds {
        &self.bounds
    }

    pub fn get_children(&self) -> &[Option<Box<QuadTree>>; 4] {
        &self.children
    }
}