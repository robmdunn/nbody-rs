use std::ffi::CString;
use std::num::NonZeroU32;
use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextAttributesBuilder, PossiblyCurrentContext},
    display::GetGlDisplay,
    prelude::*,
    surface::{Surface, SwapInterval, WindowSurface},
};
use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::HasRawWindowHandle;
use winit::{
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};
use gl::types::*;
use crate::body::Body;
use crate::tree::QuadTree;

struct Renderer {
    gl_context: PossiblyCurrentContext,
    gl_surface: Surface<WindowSurface>,
    vertex_buffer: u32,
    vertex_array: u32,
    program: u32,
    projection_loc: i32,
    color_loc: i32,
    point_size: f32,
    _window: Window,
}

const VERTEX_SHADER: &str = r#"
    #version 330 core
    layout (location = 0) in vec2 position;
    uniform mat4 projection;
    uniform float point_size;
    void main() {
        gl_Position = projection * vec4(position.xy, 0.0, 1.0);
        gl_PointSize = point_size;
    }
"#;

const FRAGMENT_SHADER: &str = r#"
    #version 330 core
    uniform vec4 color;
    out vec4 FragColor;
    void main() {
        vec2 circCoord = 2.0 * gl_PointCoord - 1.0;
        float circle = dot(circCoord, circCoord);
        if (circle > 1.0) {
            discard;
        }
        FragColor = color;
    }
"#;

static mut RENDERER: Option<Renderer> = None;

impl Renderer {
    fn new(event_loop: &EventLoop<()>, window_size: (u32, u32), point_size: f32) -> Result<Self, String> {
        let window_builder = WindowBuilder::new()
            .with_title("N-body Simulation")
            .with_inner_size(winit::dpi::LogicalSize::new(
                window_size.0 as f64,
                window_size.1 as f64,
            ));

        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_transparency(true);

        let display_builder = DisplayBuilder::new().with_window_builder(Some(window_builder));
        let (window, gl_config) = display_builder
            .build(&event_loop, template, |configs| {
                configs
                    .reduce(|accum, config| {
                        let transparency_check = config.supports_transparency().unwrap_or(false)
                            & !accum.supports_transparency().unwrap_or(false);
                        if transparency_check || config.num_samples() > accum.num_samples() {
                            config
                        } else {
                            accum
                        }
                    })
                    .unwrap()
            })
            .map_err(|e| format!("Failed to build window: {}", e))?;

        let window = window.unwrap();
        let raw_window_handle = window.raw_window_handle();

        let gl_display = gl_config.display();
        let context_attributes = ContextAttributesBuilder::new().build(Some(raw_window_handle));
        let gl_context = unsafe {
            gl_display
                .create_context(&gl_config, &context_attributes)
                .map_err(|e| format!("Failed to create context: {}", e))?
        };

        let attrs = window.build_surface_attributes(<_>::default());
        let gl_surface = unsafe {
            gl_display
                .create_window_surface(&gl_config, &attrs)
                .map_err(|e| format!("Failed to create surface: {}", e))?
        };

        let gl_context = gl_context.make_current(&gl_surface)
            .map_err(|e| format!("Failed to make context current: {}", e))?;

        gl::load_with(|s| {
            let cstr = CString::new(s).unwrap();
            gl_display.get_proc_address(&cstr)
        });

        unsafe {
            gl_surface
                .set_swap_interval(&gl_context, SwapInterval::Wait(NonZeroU32::new(1).unwrap()))
                .map_err(|e| format!("Failed to set swap interval: {}", e))?;
        }

        // Create and compile shaders
        let vertex_shader = unsafe {
            let shader = gl::CreateShader(gl::VERTEX_SHADER);
            let c_str = CString::new(VERTEX_SHADER.as_bytes()).unwrap();
            gl::ShaderSource(shader, 1, &c_str.as_ptr(), std::ptr::null());
            gl::CompileShader(shader);

            let mut success = 0;
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
            if success == 0 {
                let mut len = 0;
                gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
                let mut buffer = Vec::with_capacity(len as usize);
                buffer.set_len((len as usize) - 1);
                gl::GetShaderInfoLog(shader, len, std::ptr::null_mut(), buffer.as_mut_ptr() as *mut _);
                return Err(format!("Failed to compile vertex shader: {}", String::from_utf8_lossy(&buffer)));
            }
            shader
        };

        let fragment_shader = unsafe {
            let shader = gl::CreateShader(gl::FRAGMENT_SHADER);
            let c_str = CString::new(FRAGMENT_SHADER.as_bytes()).unwrap();
            gl::ShaderSource(shader, 1, &c_str.as_ptr(), std::ptr::null());
            gl::CompileShader(shader);
            
            let mut success = 0;
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
            if success == 0 {
                let mut len = 0;
                gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
                let mut buffer = Vec::with_capacity(len as usize);
                buffer.set_len((len as usize) - 1);
                gl::GetShaderInfoLog(shader, len, std::ptr::null_mut(), buffer.as_mut_ptr() as *mut _);
                return Err(format!("Failed to compile fragment shader: {}", String::from_utf8_lossy(&buffer)));
            }
            shader
        };

        let program = unsafe {
            let program = gl::CreateProgram();
            gl::AttachShader(program, vertex_shader);
            gl::AttachShader(program, fragment_shader);
            gl::LinkProgram(program);
            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);
            program
        };

        let projection_loc = unsafe {
            let name = CString::new("projection").unwrap();
            gl::GetUniformLocation(program, name.as_ptr())
        };

        let color_loc = unsafe {
            let name = CString::new("color").unwrap();
            gl::GetUniformLocation(program, name.as_ptr())
        };

        let mut vertex_array = 0;
        let mut vertex_buffer = 0;

        unsafe {
            gl::GenVertexArrays(1, &mut vertex_array);
            gl::BindVertexArray(vertex_array);

            gl::GenBuffers(1, &mut vertex_buffer);
            gl::BindBuffer(gl::ARRAY_BUFFER, vertex_buffer);

            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                0,
                std::ptr::null(),
            );
        }

        Ok(Renderer {
            gl_context,
            gl_surface,
            vertex_buffer,
            vertex_array,
            program,
            projection_loc,
            color_loc,
            point_size,
            _window: window,
        })
    }

    fn render(&self, bodies: &[Body], tree: &QuadTree) {
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::UseProgram(self.program);

            // Set up orthographic projection
            let bounds = tree.get_bounds();
            let scale = 1.0 / bounds.diagonal() as f32;
            let projection = [
                scale, 0.0, 0.0, 0.0,
                0.0, scale, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0f32,
            ];
            gl::UniformMatrix4fv(self.projection_loc, 1, gl::FALSE, projection.as_ptr());

            // Draw quadtree
            gl::Uniform4f(self.color_loc, 0.7, 0.7, 0.7, 0.5);
            self.draw_tree(tree);

            // Draw bodies
            gl::Uniform4f(self.color_loc, 1.0, 1.0, 1.0, 1.0);
            self.draw_bodies(bodies);

            self.gl_surface.swap_buffers(&self.gl_context).unwrap();
        }
    }

    fn draw_tree(&self, tree: &QuadTree) {
        let bounds = tree.get_bounds();
        let vertices: Vec<f32> = vec![
            bounds.min[0] as f32, bounds.min[1] as f32,
            bounds.max[0] as f32, bounds.min[1] as f32,
            bounds.max[0] as f32, bounds.max[1] as f32,
            bounds.min[0] as f32, bounds.max[1] as f32,
            bounds.min[0] as f32, bounds.min[1] as f32,
        ];

        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * std::mem::size_of::<f32>()) as GLsizeiptr,
                vertices.as_ptr() as *const _,
                gl::STREAM_DRAW,
            );

            gl::DrawArrays(gl::LINE_STRIP, 0, vertices.len() as i32 / 2);

            // Recursively draw children
            for child in tree.get_children().iter().flatten() {
                self.draw_tree(child);
            }
        }
    }

    fn draw_bodies(&self, bodies: &[Body]) {
        let vertices: Vec<f32> = bodies
            .iter()
            .flat_map(|body| [body.position[0] as f32, body.position[1] as f32])
            .collect();

        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * std::mem::size_of::<f32>()) as GLsizeiptr,
                vertices.as_ptr() as *const _,
                gl::STREAM_DRAW,
            );

            gl::PointSize(self.point_size);
            gl::DrawArrays(gl::POINTS, 0, bodies.len() as i32);
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &self.vertex_array);
            gl::DeleteBuffers(1, &self.vertex_buffer);
            gl::DeleteProgram(self.program);
        }
    }
}

pub fn init_window(event_loop: &EventLoop<()>, width: u32, height: u32, point_size: f32) -> Result<(), String> {
    unsafe {
        if RENDERER.is_some() {
            return Err("Renderer already initialized".to_string());
        }
        RENDERER = Some(Renderer::new(event_loop, (width, height), point_size)?);
    }
    Ok(())
}

pub fn draw(bodies: &[Body], tree: &QuadTree) {
    unsafe {
        if let Some(ref renderer) = RENDERER {
            renderer.render(bodies, tree);
        }
    }
}

pub fn close_window() {
    unsafe {
        RENDERER = None;
    }
}

pub fn window_open() -> bool {
    unsafe { RENDERER.is_some() }
}