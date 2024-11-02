use std::ffi::CString;
use glutin::{
    context::{PossiblyCurrentContext},
    display::GetGlDisplay,
    prelude::*,
    surface::{Surface, WindowSurface, SurfaceAttributesBuilder},
    context::{ContextApi, Version},
    config::ConfigTemplateBuilder,
};
use glutin_winit::DisplayBuilder;
use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoop,
    window::{Window, WindowAttributes},
    raw_window_handle::{HasWindowHandle, HandleError},
};
use std::{num::NonZeroU32, ptr};
use std::sync::atomic::{AtomicBool, Ordering};
use gl::types::*;
use crate::{body::Body, tree::QuadTree};

// Use atomics for safer state management
static WINDOW_OPEN: AtomicBool = AtomicBool::new(false);
static mut POINT_SIZE: f32 = 1.0;

// Shader sources
const VS_SRC: &str = r#"
    #version 330 core
    layout (location = 0) in vec2 position;
    uniform mat4 projection;
    void main() {
        gl_Position = projection * vec4(position.xy, 0.0, 1.0);
        gl_PointSize = float(gl_PointSize);
    }
"#;

const FS_SRC: &str = r#"
    #version 330 core
    uniform vec4 color;
    out vec4 FragColor;
    void main() {
        FragColor = color;
    }
"#;

struct RenderContext {
    _window: Window,
    context: PossiblyCurrentContext,
    surface: Surface<WindowSurface>,
    shader: Shader,
    vao: GLuint,
    vbo: GLuint,
}

struct Shader {
    program: GLuint,
    projection_loc: GLint,
    color_loc: GLint,
}

impl Shader {
    fn new() -> Result<Self, String> {
        unsafe {
            // Create vertex shader
            let vs = gl::CreateShader(gl::VERTEX_SHADER);
            let vs_src = CString::new(VS_SRC).unwrap();
            gl::ShaderSource(vs, 1, &vs_src.as_ptr(), ptr::null());
            gl::CompileShader(vs);
            check_shader_error(vs, "vertex")?;

            // Create fragment shader
            let fs = gl::CreateShader(gl::FRAGMENT_SHADER);
            let fs_src = CString::new(FS_SRC).unwrap();
            gl::ShaderSource(fs, 1, &fs_src.as_ptr(), ptr::null());
            gl::CompileShader(fs);
            check_shader_error(fs, "fragment")?;

            // Create and link program
            let program = gl::CreateProgram();
            gl::AttachShader(program, vs);
            gl::AttachShader(program, fs);
            gl::LinkProgram(program);
            check_shader_error(program, "program")?;

            // Clean up shaders
            gl::DeleteShader(vs);
            gl::DeleteShader(fs);

            // Get uniform locations
            let projection_str = CString::new("projection").unwrap();
            let color_str = CString::new("color").unwrap();
            let projection_loc = gl::GetUniformLocation(program, projection_str.as_ptr());
            let color_loc = gl::GetUniformLocation(program, color_str.as_ptr());

            Ok(Shader {
                program,
                projection_loc,
                color_loc,
            })
        }
    }

    fn use_program(&self) {
        unsafe {
            gl::UseProgram(self.program);
        }
    }

    fn set_color(&self, r: f32, g: f32, b: f32, a: f32) {
        unsafe {
            gl::Uniform4f(self.color_loc, r, g, b, a);
        }
    }

    fn set_projection(&self, matrix: &[f32; 16]) {
        unsafe {
            gl::UniformMatrix4fv(self.projection_loc, 1, gl::FALSE, matrix.as_ptr());
        }
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.program);
        }
    }
}

static mut RENDER_CONTEXT: Option<RenderContext> = None;

fn check_shader_error(shader: GLuint, kind: &str) -> Result<(), String> {
    unsafe {
        let mut success = gl::FALSE as GLint;
        let mut info_log = Vec::with_capacity(512);
        info_log.set_len(512);

        if kind == "program" {
            gl::GetProgramiv(shader, gl::LINK_STATUS, &mut success);
            if success != gl::TRUE as GLint {
                gl::GetProgramInfoLog(
                    shader,
                    512,
                    ptr::null_mut(),
                    info_log.as_mut_ptr() as *mut GLchar,
                );
                return Err(format!(
                    "Program linking failed: {}",
                    String::from_utf8_lossy(&info_log)
                ));
            }
        } else {
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
            if success != gl::TRUE as GLint {
                gl::GetShaderInfoLog(
                    shader,
                    512,
                    ptr::null_mut(),
                    info_log.as_mut_ptr() as *mut GLchar,
                );
                return Err(format!(
                    "{} shader compilation failed: {}",
                    kind,
                    String::from_utf8_lossy(&info_log)
                ));
            }
        }
        Ok(())
    }
}

pub fn init_window(width: u32, height: u32, point_size: f32) -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|e| format!("Failed to create event loop: {}", e))?;
    
    let mut attributes = Window::default_attributes();
    attributes.inner_size = Some(PhysicalSize::new(width, height).into());
    attributes.title = "N-body Simulation".to_string();

    let template = ConfigTemplateBuilder::new()
        .with_alpha_size(8)
        .with_transparency(true);

    let display_builder = DisplayBuilder::new()
        .with_window_attributes(Some(attributes));

    let (window, gl_config) = display_builder
        .build(&event_loop, template, |configs| {
            configs
                .reduce(|accum, config| {
                    if config.num_samples() > accum.num_samples() {
                        config
                    } else {
                        accum
                    }
                })
                .unwrap()
        })
        .map_err(|e| format!("Failed to build window: {}", e))?;

    let window = window.unwrap();
    let window_handle = window.window_handle()
        .map_err(|e| format!("Failed to get window handle: {}", e))?;

    let context_attributes = glutin::context::ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
        .build(Some(window_handle.into()));

    let surface_attributes = SurfaceAttributesBuilder::<WindowSurface>::new()
        .with_srgb(Some(true))
        .build(
            window_handle.into(),
            NonZeroU32::new(width).unwrap(),
            NonZeroU32::new(height).unwrap(),
        );

    let gl_display = gl_config.display();
    let gl_surface = unsafe {
        gl_display
            .create_window_surface(&gl_config, &surface_attributes)
            .map_err(|e| format!("Failed to create surface: {}", e))?
    };

    let context = unsafe {
        gl_display
            .create_context(&gl_config, &context_attributes)
            .map_err(|e| format!("Failed to create context: {}", e))?
            .make_current(&gl_surface)
            .map_err(|e| format!("Failed to make context current: {}", e))?
    };

    // Initialize GL
    gl::load_with(|symbol| {
        let c_str = CString::new(symbol).unwrap();
        gl_display.get_proc_address(&c_str)
    });

    unsafe {
        POINT_SIZE = point_size;

        // OpenGL initialization
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::ClearColor(0.0, 0.0, 0.1, 1.0);

        // Create and bind VAO and VBO
        let mut vao = 0;
        let mut vbo = 0;
        gl::GenVertexArrays(1, &mut vao);
        gl::GenBuffers(1, &mut vbo);
        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

        // Set up vertex attributes
        gl::EnableVertexAttribArray(0);
        gl::VertexAttribPointer(
            0,
            2,
            gl::FLOAT,
            gl::FALSE,
            0,
            ptr::null(),
        );

        // Create shader program
        let shader = Shader::new()?;

        RENDER_CONTEXT = Some(RenderContext {
            _window: window,
            context,
            surface: gl_surface,
            shader,
            vao,
            vbo,
        });

        WINDOW_OPEN.store(true, Ordering::SeqCst);
    }

    Ok(())
}

pub fn draw(bodies: &[Body], tree: &QuadTree) {
    unsafe {
        if let Some(ref context) = RENDER_CONTEXT {
            // Clear the screen
            gl::Clear(gl::COLOR_BUFFER_BIT);

            context.shader.use_program();

            // Set up    orthographic projection
            let projection = [
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ];
            context.shader.set_projection(&projection);

            // Draw quad tree
            draw_tree(tree, context);

            // Draw bodies
            draw_bodies(bodies, context);

            // Swap buffers
            context.surface
                .swap_buffers(&context.context)
                .expect("Failed to swap buffers");
        }
    }
}

fn draw_tree(tree: &QuadTree, context: &RenderContext) {
    // Set tree color
    context.shader.set_color(0.3, 0.3, 0.3, 0.5);

    // Draw tree bounds
    let bounds = tree.get_bounds();
    let vertices: Vec<f32> = vec![
        bounds.min[0] as f32, bounds.min[1] as f32,
        bounds.max[0] as f32, bounds.min[1] as f32,
        bounds.max[0] as f32, bounds.max[1] as f32,
        bounds.min[0] as f32, bounds.max[1] as f32,
        bounds.min[0] as f32, bounds.min[1] as f32,
    ];

    unsafe {
        gl::BindBuffer(gl::ARRAY_BUFFER, context.vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (vertices.len() * std::mem::size_of::<f32>()) as GLsizeiptr,
            vertices.as_ptr() as *const _,
            gl::STREAM_DRAW,
        );

        gl::DrawArrays(gl::LINE_STRIP, 0, vertices.len() as i32 / 2);

        // Recursively draw children
        for child in tree.get_children().iter().flatten() {
            draw_tree(child, context);
        }
    }
}

fn draw_bodies(bodies: &[Body], context: &RenderContext) {
    // Set body color
    context.shader.set_color(1.0, 1.0, 1.0, 1.0);

    // Prepare vertex data by converting f64 positions to f32
    let vertices: Vec<f32> = bodies.iter()
        .flat_map(|body| [
            body.position[0] as f32,
            body.position[1] as f32
        ])
        .collect();

    unsafe {
        gl::BindBuffer(gl::ARRAY_BUFFER, context.vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (vertices.len() * std::mem::size_of::<f32>()) as GLsizeiptr,
            vertices.as_ptr() as *const _,
            gl::STREAM_DRAW,
        );

        gl::PointSize(POINT_SIZE);
        gl::DrawArrays(gl::POINTS, 0, bodies.len() as i32);
    }
}

pub fn window_open() -> bool {
    WINDOW_OPEN.load(Ordering::SeqCst)
}

pub fn close_window() {
    unsafe {
        WINDOW_OPEN.store(false, Ordering::SeqCst);
        if let Some(context) = RENDER_CONTEXT.take() {
            gl::DeleteVertexArrays(1, &context.vao);
            gl::DeleteBuffers(1, &context.vbo);
        }
    }
}