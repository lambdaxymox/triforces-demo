extern crate gl;
extern crate glfw;
extern crate stb_image;
extern crate chrono;
extern crate simple_cgmath;

#[macro_use]
mod logger;

mod camera;
mod gl_helpers;

use glfw::{Action, Context, Key};
use gl::types::{GLfloat, GLint, GLsizeiptr, GLvoid, GLuint};

use gl_helpers as glh;
use simple_cgmath as math;
use camera::Camera;
use math::{Matrix4, Quaternion};

use std::mem;
use std::process;
use std::ptr;

// OpenGL extension constants.
const GL_TEXTURE_MAX_ANISOTROPY_EXT: u32 = 0x84FE;
const GL_MAX_TEXTURE_MAX_ANISOTROPY_EXT: u32 = 0x84FF;

// Log file.
const GL_LOG_FILE: &str = "gl.log";


struct GameState {
    gl_state: glh::GLState,
    camera: Camera,
}

fn create_triangle_geometry(gl_state: &mut glh::GLState) {
    let points: [f32; 9] = [
        0.0, 0.5, 0.0, -0.5, -0.5, 0.0, 0.5, -0.5, 0.0
    ];

    let mut points_vbo = 0;
    unsafe {
        gl::GenBuffers(1, &mut points_vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, points_vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER, (mem::size_of::<GLfloat>() * points.len()) as GLsizeiptr,
            points.as_ptr() as *const GLvoid, gl::STATIC_DRAW
        );
    }
    assert!(points_vbo > 0);

    let mut points_vao = 0;
    unsafe {
        gl::GenVertexArrays(1, &mut points_vao);
        gl::BindVertexArray(points_vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, points_vbo);
        gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 0, ptr::null());
        gl::EnableVertexAttribArray(0);
    }
    assert!(points_vao > 0);

    gl_state.vbo = points_vbo;
    gl_state.vao = points_vao;
}

fn create_triangle_shaders(gl_state: &mut glh::GLState) {
    let sp = glh::create_program_from_files(
        &gl_state, "shaders/triangle.vert.glsl", "shaders/triangle.frag.glsl"
    );
    assert!(sp > 0);
    gl_state.shader_program = sp;
    
    let sp_vp_loc = 0;
    assert!(sp_vp_loc > -1);

    gl_state.shader_vars.insert(String::from("vp"), sp_vp_loc);
}

fn create_camera(gl_state: &glh::GLState) -> Camera {
    let near = 0.1;
    let far = 100.0;
    let fov = 67.0;
    let aspect = gl_state.width as f32 / gl_state.height as f32;

    let cam_speed: GLfloat = 3.0;
    let cam_yaw_speed: GLfloat = 50.0;

    let fwd = math::vec4((0.0, 0.0, 1.0, 0.0));
    let rgt = math::vec4((1.0, 0.0,  0.0, 0.0));
    let up  = math::vec4((0.0, 1.0,  0.0, 0.0));
    let cam_pos = math::vec3((0.0, 0.0, 2.0));
    
    let axis = Quaternion::new(0.0, 0.0, 1.0, 0.0);

    Camera::new(near, far, fov, aspect, cam_speed, cam_yaw_speed, cam_pos, fwd, rgt, up, axis)
}

///
/// The GLFW frame buffer size callback function. This is normally set using 
/// the GLFW `glfwSetFramebufferSizeCallback` function, but instead we explicitly
/// handle window resizing in our state updates on the application side. Run this function 
/// whenever the frame buffer is resized.
///
#[inline]
fn glfw_framebuffer_size_callback(context: &mut GameState, width: u32, height: u32) {
    context.gl_state.width = width;
    context.gl_state.height = height;

    let aspect = context.gl_state.width as f32 / context.gl_state.height as f32;
    context.camera.aspect = aspect;
    context.camera.proj_mat = Matrix4::perspective(
        context.camera.fov, aspect, context.camera.near, context.camera.far
    );
}

fn init_game_state(mut gl_state: glh::GLState) -> GameState {
    let camera = create_camera(&gl_state);
    create_triangle_shaders(&mut gl_state);
    create_triangle_geometry(&mut gl_state);

    GameState {
        gl_state: gl_state,
        camera: camera,
    }
}

fn main() {
    let gl_state = match glh::start_gl(640, 480, GL_LOG_FILE) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Failed to Initialize OpenGL context. Got error:");
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    let mut context = init_game_state(gl_state);

    unsafe {
        gl::UseProgram(context.gl_state.shader_program);
    }

    unsafe {
        // Enable depth-testing.
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthFunc(gl::LESS);
        gl::Enable(gl::CULL_FACE);
        gl::CullFace(gl::BACK);
        gl::FrontFace(gl::CCW);
        gl::ClearColor(0.2, 0.2, 0.2, 1.0); // grey background to help spot mistakes
        gl::Viewport(0, 0, context.gl_state.width as i32, context.gl_state.height as i32);
    }

    while !context.gl_state.window.should_close() {
        let elapsed_seconds = glh::update_timers(&mut context.gl_state);
        glh::update_fps_counter(&mut context.gl_state);

        let (width, height) = context.gl_state.window.get_framebuffer_size();
        if (width != context.gl_state.width as i32) && (height != context.gl_state.height as i32) {
            glfw_framebuffer_size_callback(&mut context, width as u32, height as u32);
        }

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            gl::ClearColor(0.2, 0.2, 0.2, 1.0);
            gl::Viewport(0, 0, context.gl_state.width as i32, context.gl_state.height as i32);

            gl::UseProgram(context.gl_state.shader_program);
            gl::BindVertexArray(context.gl_state.vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 3);
        }

        context.gl_state.glfw.poll_events();

        // Check whether the user signaled GLFW to close the window.
        match context.gl_state.window.get_key(Key::Escape) {
            Action::Press | Action::Repeat => {
                context.gl_state.window.set_should_close(true);
            }
            _ => {}
        }
        /* ----------------------- END UPDATE GAME STATE ----------------------- */

        context.gl_state.window.swap_buffers();
    }
}
