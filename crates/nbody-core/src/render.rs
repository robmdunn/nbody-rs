use glow::*;
use std::sync::Arc;
use crate::{Body2D as Body, QuadTree};

pub struct Renderer {
    gl: Arc<Context>,
    program: Program,
    vertex_buffer: Buffer,
    vertex_array: VertexArray,
    color_location: UniformLocation,
    point_size_location: UniformLocation,
    point_size: f32,
    fixed_scale: bool,
    show_wireframe: bool,
}

impl Renderer {
    pub fn new(
        gl: Arc<Context>,
        point_size: f32,
        fixed_scale: bool,
    ) -> Result<Self, String> {
        unsafe {
            // Define shaders based on target platform
            #[cfg(target_arch = "wasm32")]
            let (vertex_shader_source, fragment_shader_source) = (
                // WebGL (GLSL ES 300)
                r#"#version 300 es
                layout (location = 0) in vec2 position;
                uniform float pointSize;
                uniform vec4 color;
                out vec4 vColor;

                void main() {
                    gl_Position = vec4(position.xy, 0.0, 1.0);
                    gl_PointSize = pointSize;
                    vColor = color;
                }
                "#,
                r#"#version 300 es
                precision mediump float;
                in vec4 vColor;
                out vec4 fragColor;

                void main() {
                    fragColor = vColor;
                }
                "#
            );

            #[cfg(not(target_arch = "wasm32"))]
            let (vertex_shader_source, fragment_shader_source) = (
                // Desktop OpenGL (GLSL 410)
                r#"#version 410
                layout (location = 0) in vec2 position;
                uniform float pointSize;
                uniform vec4 color;
                out vec4 vColor;

                void main() {
                    gl_Position = vec4(position.xy, 0.0, 1.0);
                    gl_PointSize = pointSize;
                    vColor = color;
                }
                "#,
                r#"#version 410
                in vec4 vColor;
                out vec4 fragColor;

                void main() {
                    fragColor = vColor;
                }
                "#
            );

            println!("Creating program...");

            let program = create_program(&gl, vertex_shader_source, fragment_shader_source)?;

            let vertex_array = gl.create_vertex_array()
                .map_err(|e| format!("Failed to create vertex array: {}", e))?;

            let vertex_buffer = gl.create_buffer()
                .map_err(|e| format!("Failed to create vertex buffer: {}", e))?;

            gl.bind_vertex_array(Some(vertex_array));
            gl.bind_buffer(ARRAY_BUFFER, Some(vertex_buffer));

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(
                0,          // location
                2,          // size (vec2)
                FLOAT,      // type
                false,      // normalized
                0,          // stride
                0,          // offset
            );

            let color_location = gl.get_uniform_location(program, "color")
                .ok_or_else(|| "Failed to get color uniform location".to_string())?;

            let point_size_location = gl.get_uniform_location(program, "pointSize")
                .ok_or_else(|| "Failed to get pointSize uniform location".to_string())?;

            // Initial setup
            gl.use_program(Some(program));
            gl.clear_color(0.0, 0.0, 0.1, 1.0);
            gl.enable(BLEND);
            gl.enable(PROGRAM_POINT_SIZE);
            gl.blend_func(SRC_ALPHA, ONE_MINUS_SRC_ALPHA);

            Ok(Renderer {
                gl,
                program,
                vertex_buffer,
                vertex_array,
                color_location,
                point_size_location,
                point_size,
                fixed_scale,
                show_wireframe: true,
            })
        }
    }

    pub fn set_wireframe(&mut self, show_wireframe: bool) {
        self.show_wireframe = show_wireframe;
    }

    pub fn render(&self, bodies: &[Body], tree: &QuadTree) {
        unsafe {
            self.gl.clear(COLOR_BUFFER_BIT);
            self.gl.use_program(Some(self.program));
            self.gl.bind_vertex_array(Some(self.vertex_array));

            let scale = if self.fixed_scale {
                0.8f32
            } else {
                let bounds = tree.get_bounds();
                let width = (bounds.max[0] - bounds.min[0]).abs() as f32;
                let height = (bounds.max[1] - bounds.min[1]).abs() as f32;
                1.6f32 / width.max(height)
            };

            let (center_x, center_y) = if self.fixed_scale {
                (0.0, 0.0)
            } else {
                let bounds = tree.get_bounds();
                (
                    (bounds.min[0] + bounds.max[0]) as f32 * 0.5,
                    (bounds.min[1] + bounds.max[1]) as f32 * 0.5,
                )
            };

            // Draw tree boxes with thin lines (only if enabled)
            if self.show_wireframe {
                self.gl.line_width(1.0);
                self.gl.uniform_4_f32(Some(&self.color_location), 0.3, 0.3, 0.3, 0.8);
                self.gl.uniform_1_f32(Some(&self.point_size_location), 1.0);
                self.draw_tree(tree, scale, center_x, center_y);
            }

            // Draw bodies as points
            self.gl.uniform_4_f32(Some(&self.color_location), 1.0, 1.0, 1.0, 1.0);
            self.gl.uniform_1_f32(Some(&self.point_size_location), self.point_size * scale);
            self.draw_bodies(bodies, scale, center_x, center_y);
        }
    }

    fn draw_tree(&self, tree: &QuadTree, scale: f32, center_x: f32, center_y: f32) {
        let bounds = tree.get_bounds();
        let vertices: Vec<f32> = vec![
            (bounds.min[0] as f32 - center_x) * scale, (bounds.min[1] as f32 - center_y) * scale,
            (bounds.max[0] as f32 - center_x) * scale, (bounds.min[1] as f32 - center_y) * scale,
            (bounds.max[0] as f32 - center_x) * scale, (bounds.max[1] as f32 - center_y) * scale,
            (bounds.min[0] as f32 - center_x) * scale, (bounds.max[1] as f32 - center_y) * scale,
            (bounds.min[0] as f32 - center_x) * scale, (bounds.min[1] as f32 - center_y) * scale,
        ];

        unsafe {
            self.gl.bind_buffer(ARRAY_BUFFER, Some(self.vertex_buffer));
            self.gl.buffer_data_u8_slice(
                ARRAY_BUFFER,
                std::slice::from_raw_parts(
                    vertices.as_ptr() as *const u8,
                    vertices.len() * std::mem::size_of::<f32>(),
                ),
                STREAM_DRAW,
            );

            self.gl.draw_arrays(LINE_STRIP, 0, vertices.len() as i32 / 2);

            for child in tree.get_children().iter().flatten() {
                self.draw_tree(child, scale, center_x, center_y);
            }
        }
    }

    fn draw_bodies(&self, bodies: &[Body], scale: f32, center_x: f32, center_y: f32) {
        let vertices: Vec<f32> = bodies
            .iter()
            .flat_map(|body| [
                (body.position[0] as f32 - center_x) * scale,
                (body.position[1] as f32 - center_y) * scale,
            ])
            .collect();

        unsafe {
            self.gl.bind_buffer(ARRAY_BUFFER, Some(self.vertex_buffer));
            self.gl.buffer_data_u8_slice(
                ARRAY_BUFFER,
                std::slice::from_raw_parts(
                    vertices.as_ptr() as *const u8,
                    vertices.len() * std::mem::size_of::<f32>(),
                ),
                STREAM_DRAW,
            );

            self.gl.draw_arrays(POINTS, 0, bodies.len() as i32);
        }
    }
}

fn create_program(
    gl: &Context,
    vert_source: &str,
    frag_source: &str,
) -> Result<Program, String> {
    unsafe {
        let program = gl.create_program()
            .map_err(|e| format!("Failed to create program: {}", e))?;

        let shader_sources = [
            (VERTEX_SHADER, vert_source),
            (FRAGMENT_SHADER, frag_source),
        ];

        let mut shaders = Vec::with_capacity(shader_sources.len());

        for (shader_type, shader_source) in shader_sources.iter() {
            let shader = gl.create_shader(*shader_type)
                .map_err(|e| format!("Failed to create shader: {}", e))?;

            gl.shader_source(shader, shader_source);
            gl.compile_shader(shader);

            if !gl.get_shader_compile_status(shader) {
                let error = gl.get_shader_info_log(shader);
                return Err(format!("Failed to compile shader: {}", error));
            }

            gl.attach_shader(program, shader);
            shaders.push(shader);
        }

        gl.link_program(program);

        for shader in shaders {
            gl.delete_shader(shader);
        }

        if !gl.get_program_link_status(program) {
            let error = gl.get_program_info_log(program);
            return Err(format!("Failed to link program: {}", error));
        }

        Ok(program)
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_buffer(self.vertex_buffer);
            self.gl.delete_vertex_array(self.vertex_array);
            self.gl.delete_program(self.program);
        }
    }
}

// 3D RENDERER

use crate::{Body3D, OctTree};

pub struct Camera {
    pub position: [f32; 3],
    pub target: [f32; 3],
    pub up: [f32; 3],
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    pub fn new(aspect: f32) -> Self {
        Camera {
            position: [0.0, 0.0, 10.0],  // Start looking down from above (Z is up)
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],         // Y is now "up" in screen space (toward top of screen)
            fov: 45.0_f32.to_radians(),
            aspect,
            near: 0.1,
            far: 100.0,
        }
    }

    pub fn view_matrix(&self) -> [f32; 16] {
        // Calculate camera forward, right, and up vectors
        let forward = normalize(subtract(self.target, self.position));
        let right = normalize(cross(forward, self.up));
        let up = cross(right, forward);

        // Create view matrix (inverse of camera transform)
        [
            right[0], up[0], -forward[0], 0.0,
            right[1], up[1], -forward[1], 0.0,
            right[2], up[2], -forward[2], 0.0,
            -dot(right, self.position), -dot(up, self.position), dot(forward, self.position), 1.0,
        ]
    }

    pub fn projection_matrix(&self) -> [f32; 16] {
        // Perspective projection matrix (OpenGL style)
        let tan_half_fov = (self.fov / 2.0).tan();
        let range = self.far - self.near;
        
        [
            1.0 / (self.aspect * tan_half_fov), 0.0, 0.0, 0.0,
            0.0, 1.0 / tan_half_fov, 0.0, 0.0,
            0.0, 0.0, -(self.far + self.near) / range, -2.0 * self.far * self.near / range,
            0.0, 0.0, -1.0, 0.0,
        ]
    }
}

pub struct Renderer3D {
    gl: Arc<Context>,
    program: Program,
    vertex_buffer: Buffer,
    vertex_array: VertexArray,
    color_location: UniformLocation,
    point_size_location: UniformLocation,
    mvp_location: UniformLocation,
    point_size: f32,
    camera: Camera,
    show_wireframe: bool,
}

impl Renderer3D {
    pub fn new(
        gl: Arc<Context>,
        point_size: f32,
        aspect_ratio: f32,
    ) -> Result<Self, String> {
        unsafe {
            // Define 3D shaders
            #[cfg(target_arch = "wasm32")]
            let (vertex_shader_source, fragment_shader_source) = (
                // WebGL (GLSL ES 300)
                r#"#version 300 es
                layout (location = 0) in vec3 position;
                uniform float pointSize;
                uniform vec4 color;
                uniform mat4 mvp;
                out vec4 vColor;

                void main() {
                    gl_Position = mvp * vec4(position, 1.0);
                    gl_PointSize = pointSize;
                    vColor = color;
                }
                "#,
                r#"#version 300 es
                precision mediump float;
                in vec4 vColor;
                out vec4 fragColor;

                void main() {
                    fragColor = vColor;
                }
                "#
            );

            #[cfg(not(target_arch = "wasm32"))]
            let (vertex_shader_source, fragment_shader_source) = (
                // Desktop OpenGL (GLSL 410)
                r#"#version 410
                layout (location = 0) in vec3 position;
                uniform float pointSize;
                uniform vec4 color;
                uniform mat4 mvp;
                out vec4 vColor;

                void main() {
                    gl_Position = mvp * vec4(position, 1.0);
                    gl_PointSize = pointSize;
                    vColor = color;
                }
                "#,
                r#"#version 410
                in vec4 vColor;
                out vec4 fragColor;

                void main() {
                    fragColor = vColor;
                }
                "#
            );

            println!("Creating 3D program...");

            let program = create_program(&gl, vertex_shader_source, fragment_shader_source)?;

            let vertex_array = gl.create_vertex_array()
                .map_err(|e| format!("Failed to create vertex array: {}", e))?;

            let vertex_buffer = gl.create_buffer()
                .map_err(|e| format!("Failed to create vertex buffer: {}", e))?;

            gl.bind_vertex_array(Some(vertex_array));
            gl.bind_buffer(ARRAY_BUFFER, Some(vertex_buffer));

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(
                0,          // location
                3,          // size (vec3)
                FLOAT,      // type
                false,      // normalized
                0,          // stride
                0,          // offset
            );

            let color_location = gl.get_uniform_location(program, "color")
                .ok_or_else(|| "Failed to get color uniform location".to_string())?;

            let point_size_location = gl.get_uniform_location(program, "pointSize")
                .ok_or_else(|| "Failed to get pointSize uniform location".to_string())?;

            let mvp_location = gl.get_uniform_location(program, "mvp")
                .ok_or_else(|| "Failed to get mvp uniform location".to_string())?;

            // Initial setup
            gl.use_program(Some(program));
            gl.clear_color(0.0, 0.0, 0.1, 1.0);
            gl.enable(BLEND);
            gl.enable(PROGRAM_POINT_SIZE);
            gl.enable(DEPTH_TEST);
            gl.blend_func(SRC_ALPHA, ONE_MINUS_SRC_ALPHA);

            Ok(Renderer3D {
                gl,
                program,
                vertex_buffer,
                vertex_array,
                color_location,
                point_size_location,
                mvp_location,
                point_size,
                camera: Camera::new(aspect_ratio),
                show_wireframe: true,
            })
        }
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    pub fn set_wireframe(&mut self, show_wireframe: bool) {
        self.show_wireframe = show_wireframe;
    }

    pub fn render(&self, bodies: &[Body3D], tree: &OctTree) {
        unsafe {
            self.gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
            self.gl.use_program(Some(self.program));
            self.gl.bind_vertex_array(Some(self.vertex_array));

            // Scale based on camera distance for zoom control - closer camera = larger scale (zoom in)
            let camera_distance = (self.camera.position[0].powi(2) + self.camera.position[1].powi(2) + self.camera.position[2].powi(2)).sqrt();
            let scale = 0.1 * (10.0 / camera_distance.max(1.0));
            // Use a much smaller Z scale to prevent depth clipping issues
            let z_scale = scale * 0.01; // Very small Z scale to keep points in visible range
            let mvp = [
                scale, 0.0, 0.0, 0.0,
                0.0, scale, 0.0, 0.0,
                0.0, 0.0, z_scale, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ];
            
            // TODO: Apply camera rotation manually to vertex positions instead

            // Debug camera info and MVP matrix (only print occasionally to avoid spam)
            // static mut DEBUG_COUNTER: u32 = 0;
            // unsafe {
            //     DEBUG_COUNTER += 1;
            //     if DEBUG_COUNTER % 120 == 1 { // Print every ~2 seconds at 60fps
            //         println!("3D Camera: pos=[{:.1}, {:.1}, {:.1}], target=[{:.1}, {:.1}, {:.1}]",
            //             self.camera.position[0], self.camera.position[1], self.camera.position[2],
            //             self.camera.target[0], self.camera.target[1], self.camera.target[2]
            //         );
            //         println!("3D MVP Matrix: [{:.3}, {:.3}, {:.3}, {:.3}]", mvp[0], mvp[1], mvp[2], mvp[3]);
            //         println!("               [{:.3}, {:.3}, {:.3}, {:.3}]", mvp[4], mvp[5], mvp[6], mvp[7]);
            //         println!("               [{:.3}, {:.3}, {:.3}, {:.3}]", mvp[8], mvp[9], mvp[10], mvp[11]);
            //         println!("               [{:.3}, {:.3}, {:.3}, {:.3}]", mvp[12], mvp[13], mvp[14], mvp[15]);
            //     }
            // }

            // Upload MVP matrix
            self.gl.uniform_matrix_4_f32_slice(Some(&self.mvp_location), false, &mvp);

            // Draw octree wireframe with thin lines (only if enabled)
            if self.show_wireframe {
                self.gl.line_width(1.0);
                self.gl.uniform_4_f32(Some(&self.color_location), 0.3, 0.3, 0.3, 0.8);
                self.gl.uniform_1_f32(Some(&self.point_size_location), 1.0);
                self.draw_octree(tree);
            }

            // Draw bodies as points
            self.gl.uniform_4_f32(Some(&self.color_location), 1.0, 1.0, 1.0, 1.0);
            self.gl.uniform_1_f32(Some(&self.point_size_location), self.point_size);
            self.draw_bodies_3d(bodies);
        }
    }

    fn draw_octree(&self, tree: &OctTree) {
        let bounds = tree.get_bounds();
        
        // Apply camera rotation manually to octree vertices
        let view = self.camera.view_matrix();
        
        let original_vertices = vec![
            // Front face
            bounds.min[0] as f32, bounds.min[1] as f32, bounds.max[2] as f32,
            bounds.max[0] as f32, bounds.min[1] as f32, bounds.max[2] as f32,
            bounds.max[0] as f32, bounds.max[1] as f32, bounds.max[2] as f32,
            bounds.min[0] as f32, bounds.max[1] as f32, bounds.max[2] as f32,
            bounds.min[0] as f32, bounds.min[1] as f32, bounds.max[2] as f32,
            // Back face
            bounds.min[0] as f32, bounds.min[1] as f32, bounds.min[2] as f32,
            bounds.max[0] as f32, bounds.min[1] as f32, bounds.min[2] as f32,
            bounds.max[0] as f32, bounds.max[1] as f32, bounds.min[2] as f32,
            bounds.min[0] as f32, bounds.max[1] as f32, bounds.min[2] as f32,
            bounds.min[0] as f32, bounds.min[1] as f32, bounds.min[2] as f32,
        ];
        
        let vertices: Vec<f32> = original_vertices
            .chunks(3)
            .flat_map(|chunk| {
                let pos = [chunk[0], chunk[1], chunk[2], 1.0];
                
                // Apply view matrix transformation
                let transformed_x = view[0] * pos[0] + view[4] * pos[1] + view[8] * pos[2] + view[12] * pos[3];
                let transformed_y = view[1] * pos[0] + view[5] * pos[1] + view[9] * pos[2] + view[13] * pos[3];
                let transformed_z = view[2] * pos[0] + view[6] * pos[1] + view[10] * pos[2] + view[14] * pos[3];
                
                [transformed_x, transformed_y, transformed_z]
            })
            .collect();

        unsafe {
            self.gl.bind_buffer(ARRAY_BUFFER, Some(self.vertex_buffer));
            self.gl.buffer_data_u8_slice(
                ARRAY_BUFFER,
                std::slice::from_raw_parts(
                    vertices.as_ptr() as *const u8,
                    vertices.len() * std::mem::size_of::<f32>(),
                ),
                STREAM_DRAW,
            );

            self.gl.draw_arrays(LINE_STRIP, 0, 5);
            self.gl.draw_arrays(LINE_STRIP, 5, 5);
            
            // Draw connecting lines between faces
            let original_connections = vec![
                bounds.min[0] as f32, bounds.min[1] as f32, bounds.min[2] as f32, bounds.min[0] as f32, bounds.min[1] as f32, bounds.max[2] as f32,
                bounds.max[0] as f32, bounds.min[1] as f32, bounds.min[2] as f32, bounds.max[0] as f32, bounds.min[1] as f32, bounds.max[2] as f32,
                bounds.max[0] as f32, bounds.max[1] as f32, bounds.min[2] as f32, bounds.max[0] as f32, bounds.max[1] as f32, bounds.max[2] as f32,
                bounds.min[0] as f32, bounds.max[1] as f32, bounds.min[2] as f32, bounds.min[0] as f32, bounds.max[1] as f32, bounds.max[2] as f32,
            ];
            
            let connections: Vec<f32> = original_connections
                .chunks(3)
                .flat_map(|chunk| {
                    let pos = [chunk[0], chunk[1], chunk[2], 1.0];
                    
                    // Apply view matrix transformation
                    let transformed_x = view[0] * pos[0] + view[4] * pos[1] + view[8] * pos[2] + view[12] * pos[3];
                    let transformed_y = view[1] * pos[0] + view[5] * pos[1] + view[9] * pos[2] + view[13] * pos[3];
                    let transformed_z = view[2] * pos[0] + view[6] * pos[1] + view[10] * pos[2] + view[14] * pos[3];
                    
                    [transformed_x, transformed_y, transformed_z]
                })
                .collect();

            self.gl.buffer_data_u8_slice(
                ARRAY_BUFFER,
                std::slice::from_raw_parts(
                    connections.as_ptr() as *const u8,
                    connections.len() * std::mem::size_of::<f32>(),
                ),
                STREAM_DRAW,
            );

            self.gl.draw_arrays(LINES, 0, connections.len() as i32 / 3);

            // Recursively draw children
            for child in tree.get_children().iter().flatten() {
                self.draw_octree(child);
            }
        }
    }

    fn draw_test_point(&self) {
        // Draw a single red point at origin to test rendering
        let vertices: Vec<f32> = vec![0.0, 0.0, 0.0]; // Point at origin
        
        unsafe {
            self.gl.uniform_4_f32(Some(&self.color_location), 1.0, 0.0, 0.0, 1.0); // Red
            self.gl.uniform_1_f32(Some(&self.point_size_location), 100.0); // Very large
            
            self.gl.bind_buffer(ARRAY_BUFFER, Some(self.vertex_buffer));
            self.gl.buffer_data_u8_slice(
                ARRAY_BUFFER,
                std::slice::from_raw_parts(
                    vertices.as_ptr() as *const u8,
                    vertices.len() * std::mem::size_of::<f32>(),
                ),
                STREAM_DRAW,
            );
            
            self.gl.draw_arrays(POINTS, 0, 1);
            println!("Drew test point at origin with size 100");
        }
    }

    fn draw_bodies_3d(&self, bodies: &[Body3D]) {
        // Apply camera rotation manually to vertex positions
        let view = self.camera.view_matrix();
        
        let vertices: Vec<f32> = bodies
            .iter()
            .flat_map(|body| {
                let pos = [body.position[0] as f32, body.position[1] as f32, body.position[2] as f32, 1.0];
                
                // Manually apply view matrix transformation
                let transformed_x = view[0] * pos[0] + view[4] * pos[1] + view[8] * pos[2] + view[12] * pos[3];
                let transformed_y = view[1] * pos[0] + view[5] * pos[1] + view[9] * pos[2] + view[13] * pos[3];
                let transformed_z = view[2] * pos[0] + view[6] * pos[1] + view[10] * pos[2] + view[14] * pos[3];
                
                [transformed_x, transformed_y, transformed_z]
            })
            .collect();

        // Debug output for first few bodies (only occasionally to avoid spam)
        // static mut RENDER_DEBUG_COUNTER: u32 = 0;
        // unsafe {
        //     RENDER_DEBUG_COUNTER += 1;
        //     if RENDER_DEBUG_COUNTER % 60 == 1 && bodies.len() > 0 { // Print every second at 60fps
        //         println!("3D Render: {} bodies, first body at [{:.2}, {:.2}, {:.2}]", 
        //             bodies.len(), 
        //             bodies[0].position[0], 
        //             bodies[0].position[1], 
        //             bodies[0].position[2]
        //         );
        //         if bodies.len() > 1 {
        //             println!("  Second body at [{:.2}, {:.2}, {:.2}]", 
        //                 bodies[1].position[0], 
        //                 bodies[1].position[1], 
        //                 bodies[1].position[2]
        //             );
        //         }
        //     }
        // }

        unsafe {
            self.gl.bind_buffer(ARRAY_BUFFER, Some(self.vertex_buffer));
            self.gl.buffer_data_u8_slice(
                ARRAY_BUFFER,
                std::slice::from_raw_parts(
                    vertices.as_ptr() as *const u8,
                    vertices.len() * std::mem::size_of::<f32>(),
                ),
                STREAM_DRAW,
            );

            self.gl.draw_arrays(POINTS, 0, bodies.len() as i32);
        }
    }
}

impl Drop for Renderer3D {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_buffer(self.vertex_buffer);
            self.gl.delete_vertex_array(self.vertex_array);
            self.gl.delete_program(self.program);
        }
    }
}

// Helper functions for 3D math
fn subtract(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 0.0 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        [0.0, 0.0, 0.0]
    }
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn multiply_matrices(a: [f32; 16], b: [f32; 16]) -> [f32; 16] {
    let mut result = [0.0; 16];
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                result[i * 4 + j] += a[i * 4 + k] * b[k * 4 + j];
            }
        }
    }
    result
}