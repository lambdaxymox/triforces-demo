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
use gl::types::{GLfloat, GLint, GLsizeiptr, GLuint, GLvoid};

use gl_helpers as glh;
use obj_parser as obj;
use simple_cgmath as math;

use camera::Camera;
use component::{BufferHandle, EntityID, ShaderUniformHandle, ShaderProgram, ShaderProgramHandle, ShaderSource, TextureHandle};
use math::{Matrix4, Quaternion, AsArray};

use std::mem;
use std::process;
use std::ptr;
use std::collections::HashMap;

use stb_image::image;
use stb_image::image::LoadResult;


// OpenGL extension constants.
const GL_TEXTURE_MAX_ANISOTROPY_EXT: u32 = 0x84FE;
const GL_MAX_TEXTURE_MAX_ANISOTROPY_EXT: u32 = 0x84FF;

// Log file.
const GL_LOG_FILE: &str = "gl.log";


struct EntityDatabase {
    meshes: HashMap<EntityID, obj::ObjMesh>,
    shader_sources: HashMap<EntityID, ShaderSource>,
    textures: HashMap<EntityID, TexImage2D>,
    model_matrices: HashMap<EntityID, Matrix4>,
}

impl EntityDatabase {
    fn new() -> EntityDatabase {
        EntityDatabase {
            meshes: HashMap::new(),
            shader_sources: HashMap::new(),
            textures: HashMap::new(),
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

    let mut tex_coords_vbo = 0;
    unsafe {
        gl::GenBuffers(1, &mut tex_coords_vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, tex_coords_vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER, (mem::size_of::<GLfloat>() * mesh.tex_coords.len()) as GLsizeiptr,
            mesh.tex_coords.as_ptr() as *const GLvoid, gl::STATIC_DRAW
        )
    }
    assert!(tex_coords_vbo > 0);

    let mut tex_coords_vao = 0;
    unsafe {
        gl::GenVertexArrays(1, &mut tex_coords_vao);
        gl::BindVertexArray(tex_coords_vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, tex_coords_vbo);
        gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, 0, ptr::null());
        gl::EnableVertexAttribArray(1);
    }
    assert!(tex_coords_vao > 0);

    // TODO: Place the vertex normals for Blinn-Phong shading.

    let points_handle = BufferHandle::new(points_vbo, points_vao);
    let tex_coords_handle = BufferHandle::new(tex_coords_vbo, tex_coords_vao);
    let model_mat = Matrix4::one();

    context.gl_state.buffers.insert(id, vec![points_handle, tex_coords_handle]);
    context.entities.model_matrices.insert(id, model_mat);
    context.entities.meshes.insert(id, mesh);
}


#[derive(Copy, Clone, Eq, PartialEq, Debug)]
struct Rgba {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Rgba {
    #[inline]
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Rgba {
        Rgba { r, g, b, a }
    }
}

impl Default for Rgba {
    #[inline]
    fn default() -> Rgba {
        Rgba::new(0, 0, 0, 255)
    }
}

struct TexImage2D {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub data: Vec<Rgba>,
}

impl TexImage2D {
    pub fn new(width: u32, height: u32) -> TexImage2D {
        TexImage2D {
            width: width,
            height: height,
            depth: 4,
            data: vec![Rgba::default(); (width * height) as usize],
        }
    }

    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        &self.data[0].r
    }
}

impl<'a> From<&'a image::Image<u8>> for TexImage2D {
    fn from(image: &'a image::Image<u8>) -> TexImage2D {
        let mut data = vec![];
        for chunk in image.data.chunks(4) {
            data.push(Rgba::new(chunk[0], chunk[1], chunk[2], chunk[3]));
        }

        TexImage2D {
            width: image.width as u32,
            height: image.height as u32,
            depth: image.depth as u32,
            data: data,
        }
    }
}

///
/// Load texture image.
///
fn load_image(file_name: &str) -> Result<TexImage2D, String> {
    let force_channels = 4;
    let mut image_data = match image::load_with_depth(file_name, force_channels, false) {
        LoadResult::ImageU8(image_data) => image_data,
        LoadResult::Error(_) => {
            return Err(format!("ERROR: could not load {}", file_name));
        }
        LoadResult::ImageF32(_) => {
            return Err(format!("ERROR: Tried to load an image as byte vectors, got f32: {}", file_name));
        }
    };

    let width = image_data.width;
    let height = image_data.height;

    // Check that the image size is a power of two.
    if (width & (width - 1)) != 0 || (height & (height - 1)) != 0 {
        eprintln!("WARNING: texture {} is not power-of-2 dimensions", file_name);
    }

    let width_in_bytes = 4 *width;
    let half_height = height / 2;
    for row in 0..half_height {
        for col in 0..width_in_bytes {
            let temp = image_data.data[row * width_in_bytes + col];
            image_data.data[row * width_in_bytes + col] = image_data.data[((height - row - 1) * width_in_bytes) + col];
            image_data.data[((height - row - 1) * width_in_bytes) + col] = temp;
        }
    }

    let tex_image = TexImage2D::from(&image_data);

    Ok(tex_image)
}

///
/// Load texture image into the GPU.
///
fn load_texture(tex_data: &TexImage2D, wrapping_mode: GLuint) -> Result<TextureHandle, String> {
    let mut tex = 0;
    unsafe {
        gl::GenTextures(1, &mut tex);
        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindTexture(gl::TEXTURE_2D, tex);
        gl::TexImage2D(
            gl::TEXTURE_2D, 0, gl::RGBA as i32, tex_data.width as i32, tex_data.height as i32, 0,
            gl::RGBA, gl::UNSIGNED_BYTE,
            tex_data.as_ptr() as *const GLvoid
        );
        gl::GenerateMipmap(gl::TEXTURE_2D);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, wrapping_mode as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, wrapping_mode as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR_MIPMAP_LINEAR as GLint);
    }
    assert!(tex > 0);

    let mut max_aniso = 0.0;
    unsafe {
        gl::GetFloatv(GL_MAX_TEXTURE_MAX_ANISOTROPY_EXT, &mut max_aniso);
        // Set the maximum!
        gl::TexParameterf(gl::TEXTURE_2D, GL_TEXTURE_MAX_ANISOTROPY_EXT, max_aniso);
    }

    Ok(TextureHandle::new(tex))
}

fn create_ground_plane_texture(context: &mut GameState, id: EntityID) {
    let tex_image = load_image("assets/checkerboard2.png").unwrap();
    let tex = load_texture(&tex_image, gl::CLAMP_TO_EDGE).unwrap();

    context.entities.textures.insert(id, tex_image);
    context.gl_state.textures.insert(id, tex);
}

fn create_ground_plane_shaders(context: &mut GameState, id: EntityID) {
    let sp = glh::create_program_from_files(
        &context.gl_state, "shaders/ground_plane.vert.glsl", "shaders/ground_plane.frag.glsl"
    );
    assert!(sp > 0);

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

fn create_camera(width: f32, height: f32) -> Camera {
    let near = 0.1;
    let far = 100.0;
    let fov = 67.0;
    let aspect = width / height;

    let cam_speed: GLfloat = 3.0;
    let cam_yaw_speed: GLfloat = 50.0;

    let fwd = math::vec4((0.0, 0.0, 1.0, 0.0));
    let rgt = math::vec4((1.0, 0.0,  0.0, 0.0));
    let up  = math::vec4((0.0, 1.0,  0.0, 0.0));
    let cam_pos = math::vec3((0.0, 0.0, 30.0));
    
    let axis = Quaternion::new(0.0, 0.0, 1.0, 0.0);

    Camera::new(near, far, fov, aspect, cam_speed, cam_yaw_speed, cam_pos, fwd, rgt, up, axis)
}

fn reset_camera_to_default(context: &mut GameState) {
    let width = context.gl_state.width as f32;
    let height = context.gl_state.height as f32;
    context.camera = create_camera(width, height);
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

fn init_game_state(id: EntityID) -> GameState {
    let mut gl_state = match glh::start_gl(640, 480, GL_LOG_FILE) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Failed to Initialize OpenGL context. Got error:");
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    let camera = create_camera(gl_state.width as f32, gl_state.height as f32);
    let mut context = GameState {
        gl_state: gl_state,
        camera: camera,
        entities: EntityDatabase::new(),
    };

    create_ground_plane_geometry(&mut context, id);
    create_ground_plane_shaders(&mut context, id);
    create_ground_plane_uniforms(&context, id);
    create_ground_plane_texture(&mut context, id);

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
        let elapsed_seconds = glh::update_timers(&mut context.gl_state);

        // Update the game world.
        glh::update_fps_counter(&mut context.gl_state);

        context.gl_state.glfw.poll_events();

        // Camera control keys.
        let mut cam_moved = false;
        let mut move_to = math::vec3((0.0, 0.0, 0.0));
        let mut cam_yaw = 0.0;
        let mut cam_pitch = 0.0;
        let mut cam_roll = 0.0;
        match context.gl_state.window.get_key(Key::A) {
            Action::Press | Action::Repeat => {
                move_to.x -= context.camera.speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
            }
            _ => {}
        }
        match context.gl_state.window.get_key(Key::D) {
            Action::Press | Action::Repeat => {
                move_to.x += context.camera.speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
            }
            _ => {}
        }
        match context.gl_state.window.get_key(Key::Q) {
            Action::Press | Action::Repeat => {
                move_to.y += context.camera.speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
            }
            _ => {}
        }
        match context.gl_state.window.get_key(Key::E) {
            Action::Press | Action::Repeat => {
                move_to.y -= context.camera.speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
            }
            _ => {}
        }
        match context.gl_state.window.get_key(Key::W) {
            Action::Press | Action::Repeat => {
                move_to.z -= context.camera.speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
            }
            _ => {}
        }
        match context.gl_state.window.get_key(Key::S) {
            Action::Press | Action::Repeat => {
                move_to.z += context.camera.speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
            }
            _ => {}
        }
        match context.gl_state.window.get_key(Key::Left) {
            Action::Press | Action::Repeat => {
                cam_yaw += context.camera.yaw_speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
                let q_yaw = Quaternion::from_axis_deg(cam_yaw, math::vec3(context.camera.up));
                context.camera.axis = q_yaw * &context.camera.axis;
            }
            _ => {}
        }
        match context.gl_state.window.get_key(Key::Right) {
            Action::Press | Action::Repeat => {
                cam_yaw -= context.camera.yaw_speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
                let q_yaw = Quaternion::from_axis_deg(cam_yaw, math::vec3(context.camera.up));
                context.camera.axis = q_yaw * &context.camera.axis;
            }
            _ => {}
        }
        match context.gl_state.window.get_key(Key::Up) {
            Action::Press | Action::Repeat => {
                cam_pitch += context.camera.yaw_speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
                let q_pitch = Quaternion::from_axis_deg(cam_pitch, math::vec3(context.camera.rgt));
                context.camera.axis = q_pitch * &context.camera.axis;
            }
            _ => {}
        }
        match context.gl_state.window.get_key(Key::Down) {
            Action::Press | Action::Repeat => {
                cam_pitch -= context.camera.yaw_speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
                let q_pitch = Quaternion::from_axis_deg(cam_pitch, math::vec3(context.camera.rgt));
                context.camera.axis = q_pitch * &context.camera.axis;
            }
            _ => {}
        }
        match context.gl_state.window.get_key(Key::Z) {
            Action::Press | Action::Repeat => {
                cam_roll -= context.camera.yaw_speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
                let q_roll = Quaternion::from_axis_deg(cam_roll, math::vec3(context.camera.fwd));
                context.camera.axis = q_roll * &context.camera.axis;
            }
            _ => {}
        }
        match context.gl_state.window.get_key(Key::C) {
            Action::Press | Action::Repeat => {
                cam_roll += context.camera.yaw_speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
                let q_roll = Quaternion::from_axis_deg(cam_roll, math::vec3(context.camera.fwd));
                context.camera.axis = q_roll * &context.camera.axis;
            }
            _ => {}
        }
        match context.gl_state.window.get_key(Key::Backspace) {
            Action::Press | Action::Repeat => {
                reset_camera_to_default(&mut context);
                cam_moved = true;
            }
            _ => {}
        }
        match context.gl_state.window.get_key(Key::Escape) {
            Action::Press | Action::Repeat => {
                context.gl_state.window.set_should_close(true);
            }
            _ => {}
        }

        // Update view matrix.
        if cam_moved {
            // Recalculate local axes so we can move fwd in the direction the camera is pointing.
            let rot_mat_inv = Matrix4::from(context.camera.axis);
            context.camera.fwd = rot_mat_inv * math::vec4((0.0, 0.0, -1.0, 0.0));
            context.camera.rgt = rot_mat_inv * math::vec4((1.0, 0.0,  0.0, 0.0));
            context.camera.up  = rot_mat_inv * math::vec4((0.0, 1.0,  0.0, 0.0));

            context.camera.pos += math::vec3(context.camera.fwd) * -move_to.z;
            context.camera.pos += math::vec3(context.camera.up)  *  move_to.y;
            context.camera.pos += math::vec3(context.camera.rgt) *  move_to.x;

            let trans_mat_inv = Matrix4::from_translation(context.camera.pos);

            context.camera.rot_mat = rot_mat_inv.inverse();
            context.camera.trans_mat = trans_mat_inv.inverse();
            context.camera.view_mat = context.camera.rot_mat * context.camera.trans_mat;

            let gp_sp = &context.gl_state.shaders[&id];
            let gp_view_mat_loc = gp_sp.uniforms["view_mat"];
            unsafe {
                gl::UseProgram(gp_sp.handle.into());
                gl::UniformMatrix4fv(gp_view_mat_loc.into(), 1, gl::FALSE, context.camera.view_mat.as_ptr());
            }
        }
:
        // Render the results.
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            gl::ClearColor(0.2, 0.2, 0.2, 1.0);
        }

        let (width, height) = context.gl_state.window.get_framebuffer_size();
        if (width != context.gl_state.width as i32) && (height != context.gl_state.height as i32) {
            glfw_framebuffer_size_callback(&mut context, width as u32, height as u32);
        }

        unsafe {
            gl::Viewport(0, 0, context.gl_state.width as i32, context.gl_state.height as i32);

            gl::UseProgram(context.gl_state.shaders[&id].handle.into());
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, context.gl_state.textures[&id].into());
            gl::BindVertexArray(context.gl_state.buffers[&id][0].vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 12);
        }
        
        // Send the results to the output.
        context.gl_state.window.swap_buffers();
    }
}
