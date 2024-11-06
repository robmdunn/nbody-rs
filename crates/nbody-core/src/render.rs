use glow::*;
use std::sync::Arc;
use crate::{Body, QuadTree};

pub struct Renderer {
    gl: Arc<Context>,
    program: Program,
    vertex_buffer: Buffer,
    vertex_array: VertexArray,
    color_location: UniformLocation,
    point_size_location: UniformLocation,
    point_size: f32,
    fixed_scale: bool,
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
            })
        }
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

            // Draw tree boxes with thin lines
            self.gl.line_width(1.0);
            self.gl.uniform_4_f32(Some(&self.color_location), 0.3, 0.3, 0.3, 0.8);
            self.gl.uniform_1_f32(Some(&self.point_size_location), 1.0);
            self.draw_tree(tree, scale, center_x, center_y);

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