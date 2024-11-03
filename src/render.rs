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

pub struct Renderer {
    gl_context: PossiblyCurrentContext,
    gl_surface: Surface<WindowSurface>,
    vertex_buffer: u32,
    vertex_array: u32,
    program: u32,
    color_loc: i32,
    point_size: f32,
    fixed_scale: bool,
    _window: Window,
}

const VERTEX_SHADER: &str = r#"
    #version 330 core
    layout (location = 0) in vec2 position;
    void main() {
        gl_Position = vec4(position.xy, 0.0, 1.0);
        gl_PointSize = 1.0;
    }
"#;

const FRAGMENT_SHADER: &str = r#"
    #version 330 core
    uniform vec4 color;
    out vec4 FragColor;
    void main() {
        FragColor = color;
    }
"#;

static mut RENDERER: Option<Renderer> = None;

impl Renderer {
    fn new(event_loop: &EventLoop<()>, window_size: (u32, u32), point_size: f32, fixed_scale: bool) -> Result<Self, String> {
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
            .build(event_loop, template, |configs| {
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

        let gl_context = gl_context
            .make_current(&gl_surface)
            .map_err(|e| format!("Failed to make context current: {}", e))?;

        gl::load_with(|s| {
            let cstr = CString::new(s).unwrap();
            gl_display.get_proc_address(&cstr)
        });

        gl_surface
            .set_swap_interval(&gl_context, SwapInterval::Wait(NonZeroU32::new(1).unwrap()))
            .map_err(|e| format!("Failed to set swap interval: {}", e))?;

        unsafe {
            // Basic OpenGL setup
            gl::Disable(gl::DEPTH_TEST);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::ClearColor(0.0, 0.0, 0.1, 1.0);

            println!("Creating shaders...");
            
            // Create and compile shaders
            let vertex_shader = compile_shader(gl::VERTEX_SHADER, VERTEX_SHADER)?;
            let fragment_shader = compile_shader(gl::FRAGMENT_SHADER, FRAGMENT_SHADER)?;

            println!("Shaders compiled successfully. Creating program...");

            // Create and link program
            let program = gl::CreateProgram();
            gl::AttachShader(program, vertex_shader);
            gl::AttachShader(program, fragment_shader);
            gl::LinkProgram(program);

            // Check for linking errors
            let mut success = 0;
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);
            if success == 0 {
                let mut len = 0;
                gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
                let mut buffer = vec![0u8; len as usize];
                gl::GetProgramInfoLog(
                    program,
                    len,
                    std::ptr::null_mut(),
                    buffer.as_mut_ptr() as *mut _
                );
                return Err(String::from_utf8_lossy(&buffer).into_owned());
            }

            println!("Program linked successfully.");

            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);

            // Create and set up VAO/VBO first
            let mut vertex_array = 0;
            let mut vertex_buffer = 0;

            gl::GenVertexArrays(1, &mut vertex_array);
            gl::BindVertexArray(vertex_array);

            gl::GenBuffers(1, &mut vertex_buffer);
            gl::BindBuffer(gl::ARRAY_BUFFER, vertex_buffer);

            // Set up vertex attributes ONCE during initialization
            gl::VertexAttribPointer(
                0,  // location = 0 in shader
                2,  // vec2
                gl::FLOAT,
                gl::FALSE,
                0,
                std::ptr::null()
            );
            gl::EnableVertexAttribArray(0);

            // Now get uniform locations
            let name = CString::new("color").unwrap();
            let color_loc = gl::GetUniformLocation(program, name.as_ptr());

            // Use the program before setting uniforms
            gl::UseProgram(program);

            // Print debug info
            println!("Initialized OpenGL objects:");
            println!("  Program: {}", program);
            println!("  VAO: {}", vertex_array);
            println!("  VBO: {}", vertex_buffer);
            println!("  Color loc: {}", color_loc);

            if let Some(error) = get_gl_error() {
                println!("OpenGL error during initialization: {}", error);
            }
            
            Ok(Renderer {
                gl_context,
                gl_surface,
                vertex_buffer,
                vertex_array,
                program,
                color_loc,
                point_size,
                fixed_scale,
                _window: window,
            })
        }
    }


    fn render(&self, bodies: &[Body], tree: &QuadTree) {
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::UseProgram(self.program);
            gl::BindVertexArray(self.vertex_array);

            let scale = if self.fixed_scale {
                0.8f32  // Fixed scale that maps [-1,1] to [-0.8,0.8]
            } else {
                // Dynamic scale based on current bounds
                let bounds = tree.get_bounds();
                let width = (bounds.max[0] - bounds.min[0]).abs() as f32;
                let height = (bounds.max[1] - bounds.min[1]).abs() as f32;
                1.6f32 / width.max(height)
            };

            // Center offset only needed for dynamic scaling
            let (center_x, center_y) = if self.fixed_scale {
                (0.0, 0.0)
            } else {
                let bounds = tree.get_bounds();
                (
                    (bounds.min[0] + bounds.max[0]) as f32 * 0.5,
                    (bounds.min[1] + bounds.max[1]) as f32 * 0.5,
                )
            };

            // Draw the tree boxes
            gl::LineWidth(1.0);
            gl::Uniform4f(self.color_loc, 0.3, 0.3, 0.3, 0.8);
            self.draw_tree(tree, scale, center_x, center_y);

            // Draw bodies
            gl::PointSize(self.point_size);
            gl::Uniform4f(self.color_loc, 1.0, 1.0, 1.0, 1.0);
            self.draw_bodies(bodies, scale, center_x, center_y);

            self.gl_surface.swap_buffers(&self.gl_context).unwrap();
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
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * std::mem::size_of::<f32>()) as GLsizeiptr,
                vertices.as_ptr() as *const _,
                gl::STREAM_DRAW,
            );

            gl::DrawArrays(gl::LINE_STRIP, 0, vertices.len() as i32 / 2);

            // Draw children
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
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * std::mem::size_of::<f32>()) as GLsizeiptr,
                vertices.as_ptr() as *const _,
                gl::STREAM_DRAW,
            );

            gl::DrawArrays(gl::POINTS, 0, bodies.len() as i32);
        }
    }
}


fn get_gl_error() -> Option<GLenum> {
    unsafe {
        let error = gl::GetError();
        if error != gl::NO_ERROR {
            Some(error)
        } else {
            None
        }
    }
}

fn compile_shader(shader_type: GLenum, source: &str) -> Result<GLuint, String> {
    unsafe {
        let shader = gl::CreateShader(shader_type);
        let c_str = CString::new(source.as_bytes()).unwrap();
        gl::ShaderSource(shader, 1, &c_str.as_ptr(), std::ptr::null());
        gl::CompileShader(shader);

        let mut success = 0;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
        if success == 0 {
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let mut buffer = vec![0u8; len as usize];
            gl::GetShaderInfoLog(
                shader,
                len,
                std::ptr::null_mut(),
                buffer.as_mut_ptr() as *mut _
            );
            return Err(String::from_utf8_lossy(&buffer).into_owned());
        }
        Ok(shader)
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.vertex_buffer);
            gl::DeleteVertexArrays(1, &self.vertex_array);
            gl::DeleteProgram(self.program);
        }
    }
}

pub fn init_window(
    event_loop: &EventLoop<()>, 
    width: u32, 
    height: u32, 
    point_size: f32,
    fixed_scale: bool
) -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::new(event_loop, (width, height), point_size, fixed_scale)?;
    unsafe {
        RENDERER = Some(renderer);
    }
    Ok(())
}

pub fn window_open() -> bool {
    unsafe { RENDERER.is_some() }
}

pub fn draw(bodies: &[Body], tree: &QuadTree) {
    unsafe {
        if let Some(ref renderer) = RENDERER {
            renderer.render(bodies, tree);
        }
    }
}