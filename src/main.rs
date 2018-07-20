extern crate chrono;
extern crate gl;
extern crate glfw;
extern crate stb_image;
extern crate simple_cgmath;

#[macro_use]
extern crate scan_fmt;

#[macro_use]
mod logger;

mod camera;
mod gl_helpers;
mod component;
mod obj_parser;

use glfw::{Action, Context, Key};
use gl::types::{GLfloat, GLsizeiptr, GLvoid};

use gl_helpers as glh;
use obj_parser as obj;
use simple_cgmath as math;

use camera::Camera;
use component::{BufferHandle, EntityID, ShaderUniformHandle, ShaderProgram, ShaderProgramHandle, ShaderSource};
use math::{Matrix4, Quaternion, AsArray};
use obj::ObjMesh;

use std::mem;
use std::process;
use std::ptr;
use std::collections::HashMap;


// OpenGL extension constants.
const GL_TEXTURE_MAX_ANISOTROPY_EXT: u32 = 0x84FE;
const GL_MAX_TEXTURE_MAX_ANISOTROPY_EXT: u32 = 0x84FF;

// Log file.
const GL_LOG_FILE: &str = "gl.log";


struct EntityDatabase {
    meshes: HashMap<EntityID, ObjMesh>,
    shader_sources: HashMap<EntityID, ShaderSource>,
    textures: HashMap<EntityID, ProceduralTexture>,
    model_matrices: HashMap<EntityID, Matrix4>,
}

impl EntityDatabase {
    fn new() -> EntityDatabase {
        EntityDatabase {
            meshes: HashMap::new(),
            shader_sources: HashMap::new(),
            model_matrices: HashMap::new(),
        }
    }
}

struct GameState {
    gl_state: glh::GLState,
    camera: Camera,
    entities: EntityDatabase,
}

fn create_ground_plane_geometry(context: &mut GameState, id: EntityID) {
    let mesh = obj::load_obj_file("assets/ground_plane.obj").unwrap();

    let mut points_vbo = 0;
    unsafe {
        gl::GenBuffers(1, &mut points_vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, points_vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER, (mem::size_of::<GLfloat>() * mesh.points.len()) as GLsizeiptr,
            mesh.points.as_ptr() as *const GLvoid, gl::STATIC_DRAW
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

    let handle = BufferHandle::new(points_vbo, points_vao);
    let model_mat = Matrix4::one();

    context.gl_state.buffers.insert(id, vec![handle]);
    context.entities.model_matrices.insert(id, model_mat);
    context.entities.meshes.insert(id, mesh);
}

/* ---------------- PROCEDURAL TEXTURE -------------------------- */
#[derive(Copy, Clone, Eq, PartialEq)]
struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

impl Rgb {
    #[inline]
    pub fn new(r: u8, g: u8, b: u8) -> Rgb {
        Rgb { r, g, b }
    }

    #[inline]
    pub fn zero() -> Rgb {
        Rgb { r: 0, g: 0, b: 0 }
    }
}

struct ProceduralTexture {
    pub width: u32,
    pub height: u32,
    pub buffer: Vec<Rgb>,
}

impl ProceduralTexture {
    pub fn new(width: u32, height: u32) -> ProceduralTexture {
        ProceduralTexture {
            width: width,
            height: height,
            buffer: vec![Rgb::zero(); (width * height) as usize],
        }
    }

    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        &self.buffer[0].r
    }
}

fn generate_checkerboard_texture(
    context: &mut GameState, width: u32, height: u32,
    c0: Rgb, c1: Rgb, tile_count: usize) -> ProceduralTexture {

    let mut texture = ProceduralTexture::new(width, height);
    for i in 0..((height * width) as usize) {
        texture.buffer[i] = c1;
    }

    texture
}

/* ------------------ END PROCEDURAL TEXTURE -------------------- */

fn create_ground_plane_texture(context: &mut GameState, id: EntityID) {

}

fn create_ground_plane_shaders(context: &mut GameState, id: EntityID) {
    let sp = glh::create_program_from_files(
        &context.gl_state, "shaders/ground_plane.vert.glsl", "shaders/ground_plane.frag.glsl"
    );
    assert!(sp > 0);
    let mut sp_vp_loc = 0;
    assert!(sp_vp_loc > -1);

    let sp_model_mat_loc = unsafe {
        gl::GetUniformLocation(sp, "model_mat".as_ptr() as *const i8)
    };
    assert!(sp_model_mat_loc > -1);

    let sp_view_mat_loc = unsafe {
        gl::GetUniformLocation(sp, "view_mat".as_ptr() as *const i8)
    };
    assert!(sp_view_mat_loc > -1);

    let sp_proj_mat_loc = unsafe {
        gl::GetUniformLocation(sp, "proj_mat".as_ptr() as *const i8)
    };
    assert!(sp_proj_mat_loc > -1);

    let mut shader = ShaderProgram::new(ShaderProgramHandle::from(sp));
    shader.uniforms.insert(String::from("model_mat"), ShaderUniformHandle::from(sp_model_mat_loc));
    shader.uniforms.insert(String::from("view_mat"), ShaderUniformHandle::from(sp_view_mat_loc));
    shader.uniforms.insert(String::from("proj_mat"), ShaderUniformHandle::from(sp_proj_mat_loc));

    context.gl_state.shaders.insert(id, shader);
}

fn create_ground_plane_uniforms(context: &GameState, id: EntityID) {
    let shader = &context.gl_state.shaders[&id];
    unsafe {
        gl::UseProgram(shader.handle.into());
        gl::UniformMatrix4fv(shader.uniforms["model_mat"].into(), 1, gl::FALSE, context.entities.model_matrices[&id].as_ptr());
        gl::UniformMatrix4fv(shader.uniforms["view_mat"].into(), 1, gl::FALSE, context.camera.view_mat.as_ptr());
        gl::UniformMatrix4fv(shader.uniforms["proj_mat"].into(), 1, gl::FALSE, context.camera.proj_mat.as_ptr());
    }
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
    let cam_pos = math::vec3((0.0, 0.0, 20.0));
    
    let axis = Quaternion::new(0.0, 0.0, 1.0, 0.0);

    Camera::new(near, far, fov, aspect, cam_speed, cam_yaw_speed, cam_pos, fwd, rgt, up, axis)
}

///
/// The GLFW frame buffer size callback function. This is normally set using 
/// the GLFW `glfwSetFramebufferSizeCallback` function, but instead we explicitly
/// handle window resizing in our state updates on the application side. Run this function 
/// whenever the size of the viewport changes.
///
#[inline]
fn glfw_framebuffer_size_callback(context: &mut GameState, width: u32, height: u32) {
    context.gl_state.width = width;
    context.gl_state.height = height;

    let aspect = context.gl_state.width as f32 / context.gl_state.height as f32;
    context.camera.aspect = aspect;
    context.camera.proj_mat = math::perspective((
        context.camera.fov, aspect, context.camera.near, context.camera.far
    ));
}

fn render(context: &mut GameState, id: EntityID) {
    let (width, height) = context.gl_state.window.get_framebuffer_size();
    if (width != context.gl_state.width as i32) && (height != context.gl_state.height as i32) {
        glfw_framebuffer_size_callback(context, width as u32, height as u32);
    }

    unsafe {
        gl::Viewport(0, 0, context.gl_state.width as i32, context.gl_state.height as i32);

        gl::UseProgram(context.gl_state.shaders[&id].handle.into());
        gl::BindVertexArray(context.gl_state.buffers[&id][0].vao);
        gl::DrawArrays(gl::TRIANGLES, 0, 12);
    }
}

fn init_game_state(id: EntityID) -> GameState {
    let mut gl_state = match glh::start_gl(640, 480, GL_LOG_FILE) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Failed to Initialize OpenGL context. Got error:");
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    let camera = create_camera(&gl_state);
    let mut context = GameState {
        gl_state: gl_state,
        camera: camera,
        entities: EntityDatabase::new(),
    };

    create_ground_plane_geometry(&mut context, id);
    create_ground_plane_shaders(&mut context, id);
    create_ground_plane_uniforms(&context, id);

    context
}

fn main() {
    let id = EntityID::new(0);
    let mut context = init_game_state(id);

    unsafe {
        gl::UseProgram(context.gl_state.shaders[&id].handle.into());
    }

    unsafe {
        // Enable depth testing.
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthFunc(gl::LESS);
        gl::Enable(gl::CULL_FACE);
        gl::CullFace(gl::BACK);
        gl::FrontFace(gl::CCW);
        // Gray background.
        gl::ClearColor(0.2, 0.2, 0.2, 1.0);
        gl::Viewport(0, 0, context.gl_state.width as i32, context.gl_state.height as i32);
    }

    /* --------------------------- GAME LOOP ------------------------------- */
    while !context.gl_state.window.should_close() {
        // Check input.
        glh::update_timers(&mut context.gl_state);

        context.gl_state.glfw.poll_events();
        match context.gl_state.window.get_key(Key::Escape) {
            Action::Press | Action::Repeat => {
                context.gl_state.window.set_should_close(true);
            }
            _ => {}
        }

        // Update the game world.
        glh::update_fps_counter(&mut context.gl_state);
        
        // Render the results.
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            gl::ClearColor(0.2, 0.2, 0.2, 1.0);
        }
        render(&mut context, id);
        
        // Send the results to the output.
        context.gl_state.window.swap_buffers();
    }
}
