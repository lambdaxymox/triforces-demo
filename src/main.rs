extern crate chrono;
extern crate glfw;
extern crate stb_image;
extern crate cgmath;
extern crate wavefront;
extern crate serde;
extern crate toml;

#[macro_use]
extern crate serde_derive;

mod gl {
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

#[macro_use]
mod logger;

mod camera;
mod config;
mod gl_helpers;
mod component;
mod obj;
mod lights;
mod texture;

use glfw::{Action, Context, Key};
use gl::types::{
    GLfloat, GLint, GLsizeiptr, GLuint, GLvoid
};

use gl_helpers as glh;
use cgmath as math;

use camera::Camera;
use config::Config;
use component::{
    BufferHandle, EntityID,
    ShaderUniformHandle, ShaderProgram, ShaderProgramHandle, ShaderSource,
    TextureHandle
};
use math::{Matrix4, Quaternion, AsArray};
use lights::PointLight;
use texture::TexImage2D;

use std::mem;
use std::path::{Path, PathBuf};
use std::process;
use std::ptr;
use std::collections::HashMap;


// OpenGL extension constants.
const GL_TEXTURE_MAX_ANISOTROPY_EXT: u32 = 0x84FE;
const GL_MAX_TEXTURE_MAX_ANISOTROPY_EXT: u32 = 0x84FF;

const CONFIG_FILE: &str = "config/config.toml";


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

struct GameContext {
    config: Config,
    gl: glh::GLState,
    camera: Camera,
    light: PointLight,
    entities: EntityDatabase,
}

impl GameContext {
    fn asset_file<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        Path::new(&self.config.asset_path).join(path)
    }

    fn shader_file<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        let shader_path = Path::new(&self.config.shader_path);
        let shader_version = Path::new(&self.config.shader_version);
        let file_path = shader_path.join(shader_version).join(path);

        file_path
    }

}

fn create_light() -> PointLight {
    let ambient = math::vec3((0.3, 0.3, 0.3));
    let diffuse = math::vec3((0.7, 0.7, 0.7));
    let specular = math::vec3((1.0, 1.0, 1.0));
    let specular_exponent = 100.0;
    let light_pos = math::vec3((5.0, -5.0, 25.0));

    PointLight::new(ambient, diffuse, specular, specular_exponent, light_pos)
}

fn create_camera(width: f32, height: f32) -> Camera {
    let near = 0.1;
    let far = 100.0;
    let fov = 67.0;
    let aspect = width / height;

    let cam_speed: GLfloat = 5.0;
    let cam_yaw_speed: GLfloat = 50.0;

    let fwd = math::vec4((0.0, 0.0, 1.0, 0.0));
    let rgt = math::vec4((1.0, 0.0, 0.0, 0.0));
    let up  = math::vec4((0.0, 1.0, 0.0, 0.0));
    let cam_pos = math::vec3((0.0, 0.0, 10.0));

    let axis = Quaternion::new(0.0, 0.0, 0.0, -1.0);

    Camera::new(near, far, fov, aspect, cam_speed, cam_yaw_speed, cam_pos, fwd, rgt, up, axis)
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

fn create_triforce_lights(context: &mut GameContext, id: EntityID) {
    let shader = context.gl.shaders[&id].handle.into();

    let ubo_index = unsafe { gl::GetUniformBlockIndex(shader, glh::gl_str("PointLight").as_ptr()) };
    assert!(ubo_index != gl::INVALID_INDEX);

    let mut ubo_size = 0;
    unsafe {
        gl::GetActiveUniformBlockiv(shader, ubo_index, gl::UNIFORM_BLOCK_DATA_SIZE, &mut ubo_size)
    };
    assert!(ubo_size > 0);

    let light = &context.light;

    let mut indices = [0; 5];
    let mut sizes = [0; 5];
    let mut offsets = [0; 5];
    let mut types = [0; 5];
    unsafe {
        gl::GetActiveUniformBlockiv(shader, ubo_index, gl::UNIFORM_BLOCK_ACTIVE_UNIFORM_INDICES, indices.as_mut_ptr());
        gl::GetActiveUniformsiv(shader, 5, indices.as_ptr() as *const u32, gl::UNIFORM_OFFSET, offsets.as_mut_ptr());
        gl::GetActiveUniformsiv(shader, 5, indices.as_ptr() as *const u32, gl::UNIFORM_SIZE, sizes.as_mut_ptr());
        gl::GetActiveUniformsiv(shader, 5, indices.as_ptr() as *const u32, gl::UNIFORM_TYPE, types.as_mut_ptr());
    }

    let mut buffer = vec![0 as u8; ubo_size as usize];
    unsafe {
        ptr::copy(&light.ambient, mem::transmute(&mut buffer[offsets[0] as usize]), 1);
        ptr::copy(&light.diffuse, mem::transmute(&mut buffer[offsets[1] as usize]), 1);
        ptr::copy(&light.specular, mem::transmute(&mut buffer[offsets[2] as usize]), 1);
        ptr::copy(&light.specular_exponent, mem::transmute(&mut buffer[offsets[3] as usize]), 1);
        ptr::copy(&light.position, mem::transmute(&mut buffer[offsets[4] as usize]), 1);
    }

    let mut ubo = 0;
    unsafe {
        gl::GenBuffers(1, &mut ubo);
        gl::BindBuffer(gl::UNIFORM_BUFFER, ubo);
        gl::BufferData(
            gl::UNIFORM_BUFFER, ubo_size as GLsizeiptr,
            buffer.as_ptr() as *const GLvoid, gl::STATIC_DRAW
        );
        gl::BindBufferBase(gl::UNIFORM_BUFFER, ubo_index, ubo);
    }
    assert!(ubo > 0);

    let ubo_handle = BufferHandle::new(ubo, 0);
    let mut buffers = (context.gl.buffers[&id]).clone();
    buffers.push(ubo_handle);
    context.gl.buffers.insert(id, buffers);
}

fn create_ground_plane_geometry(context: &mut GameContext, id: EntityID) {
    let mesh = obj::load_file(&context.asset_file("ground_plane.obj")).unwrap();
    let shader = context.gl.shaders[&id].handle.into();

    let points_loc = unsafe { gl::GetAttribLocation(shader, glh::gl_str("v_pos").as_ptr()) };
    assert!(points_loc > -1);
    let points_loc = points_loc as u32;

    let tex_coords_loc = unsafe { gl::GetAttribLocation(shader, glh::gl_str("v_tex").as_ptr()) };
    assert!(tex_coords_loc > -1);
    let tex_coords_loc = tex_coords_loc as u32;

    let mut points_vbo = 0;
    unsafe {
        gl::GenBuffers(1, &mut points_vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, points_vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER, (3 * mem::size_of::<GLfloat>() * mesh.points.len()) as GLsizeiptr,
            mesh.points.as_ptr() as *const GLvoid, gl::STATIC_DRAW
        );
    }
    assert!(points_vbo > 0);

    let mut tex_coords_vbo = 0;
    unsafe {
        gl::GenBuffers(1, &mut tex_coords_vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, tex_coords_vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER, (2 * mem::size_of::<GLfloat>() * mesh.tex_coords.len()) as GLsizeiptr,
            mesh.tex_coords.as_ptr() as *const GLvoid, gl::STATIC_DRAW
        )
    }
    assert!(tex_coords_vbo > 0);

    let mut vao = 0;
    unsafe {
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, points_vbo);
        gl::VertexAttribPointer(points_loc, 3, gl::FLOAT, gl::FALSE, 0, ptr::null());
        gl::BindBuffer(gl::ARRAY_BUFFER, tex_coords_vbo);
        gl::VertexAttribPointer(tex_coords_loc, 2, gl::FLOAT, gl::FALSE, 0, ptr::null());
        gl::EnableVertexAttribArray(points_loc);
        gl::EnableVertexAttribArray(tex_coords_loc);
    }
    assert!(vao > 0);

    let points_handle = BufferHandle::new(points_vbo, vao);
    let tex_coords_handle = BufferHandle::new(tex_coords_vbo, vao);
    let model_mat = Matrix4::one();

    context.gl.buffers.insert(id, vec![points_handle, tex_coords_handle]);
    context.entities.model_matrices.insert(id, model_mat);
    context.entities.meshes.insert(id, mesh);
}

fn create_ground_plane_texture(context: &mut GameContext, id: EntityID) {
    let tex_image = texture::load_file(&context.asset_file("ground_plane.png")).unwrap();
    let tex = load_texture(&tex_image, gl::CLAMP_TO_EDGE).unwrap();

    context.entities.textures.insert(id, tex_image);
    context.gl.textures.insert(id, tex);
}

fn create_ground_plane_shaders(context: &mut GameContext, id: EntityID) {
    let sp = glh::create_program_from_files(
        &context.gl,
        &context.shader_file("ground_plane.vert.glsl"),
        &context.shader_file("ground_plane.frag.glsl")
    ).unwrap();
    assert!(sp > 0);

    let sp_model_mat_loc = unsafe {
        gl::GetUniformLocation(sp, glh::gl_str("model_mat").as_ptr())
    };
    assert!(sp_model_mat_loc > -1);

    let sp_view_mat_loc = unsafe {
        gl::GetUniformLocation(sp, glh::gl_str("view_mat").as_ptr())
    };
    assert!(sp_view_mat_loc > -1);

    let sp_proj_mat_loc = unsafe {
        gl::GetUniformLocation(sp, glh::gl_str("proj_mat").as_ptr())
    };
    assert!(sp_proj_mat_loc > -1);

    let mut shader = ShaderProgram::new(ShaderProgramHandle::from(sp));
    shader.uniforms.insert(String::from("model_mat"), ShaderUniformHandle::from(sp_model_mat_loc));
    shader.uniforms.insert(String::from("view_mat"), ShaderUniformHandle::from(sp_view_mat_loc));
    shader.uniforms.insert(String::from("proj_mat"), ShaderUniformHandle::from(sp_proj_mat_loc));

    context.gl.shaders.insert(id, shader);
}

fn create_ground_plane_uniforms(context: &GameContext, id: EntityID) {
    let shader = &context.gl.shaders[&id];
    unsafe {
        gl::UseProgram(shader.handle.into());
        gl::UniformMatrix4fv(shader.uniforms["model_mat"].into(), 1, gl::FALSE, context.entities.model_matrices[&id].as_ptr());
        gl::UniformMatrix4fv(shader.uniforms["view_mat"].into(), 1, gl::FALSE, context.camera.view_mat.as_ptr());
        gl::UniformMatrix4fv(shader.uniforms["proj_mat"].into(), 1, gl::FALSE, context.camera.proj_mat.as_ptr());
    }
}

///
/// Load the geometry for the triforce.
///
fn create_triforce_geometry(context: &mut GameContext, id: EntityID, model_mat: Matrix4) {
    let mesh = obj::load_file(&context.asset_file("triangle.obj")).unwrap();
    let shader = context.gl.shaders[&id].handle.into();

    let points_loc = unsafe { gl::GetAttribLocation(shader, glh::gl_str("v_pos").as_ptr()) };
    assert!(points_loc > -1);
    let points_loc = points_loc as u32;

    let tex_coords_loc = unsafe { gl:: GetAttribLocation(shader, glh::gl_str("v_tex").as_ptr()) };
    assert!(tex_coords_loc > -1);
    let tex_coords_loc = tex_coords_loc as u32;

    let normals_loc = unsafe { gl::GetAttribLocation(shader, glh::gl_str("v_norm").as_ptr()) };
    assert!(normals_loc > -1);
    let normals_loc = normals_loc as u32;

    let mut points_vbo = 0;
    unsafe {
        gl::GenBuffers(1, &mut points_vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, points_vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER, (3 * mem::size_of::<GLfloat>() * mesh.points.len()) as GLsizeiptr,
            mesh.points.as_ptr() as *const GLvoid, gl::STATIC_DRAW
        );
    }
    assert!(points_vbo > 0);

    let mut tex_coords_vbo = 0;
    unsafe {
        gl::GenBuffers(1, &mut tex_coords_vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, tex_coords_vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER, (2 * mem::size_of::<GLfloat>() * mesh.tex_coords.len()) as GLsizeiptr,
            mesh.tex_coords.as_ptr() as *const GLvoid, gl::STATIC_DRAW
        );
    }
    assert!(tex_coords_vbo > 0);

    let mut normals_vbo = 0;
    unsafe {
        gl::GenBuffers(1, &mut normals_vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, normals_vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER, (3 * mem::size_of::<GLfloat>() * mesh.normals.len()) as GLsizeiptr,
            mesh.normals.as_ptr() as *const GLvoid, gl::STATIC_DRAW
        );
    }
    assert!(normals_vbo > 0);

    let mut vao = 0;
    unsafe {
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, points_vbo);
        gl::VertexAttribPointer(points_loc, 3, gl::FLOAT, gl::FALSE, 0, ptr::null());
        gl::EnableVertexAttribArray(points_loc);
        gl::BindBuffer(gl::ARRAY_BUFFER, tex_coords_vbo);
        gl::VertexAttribPointer(tex_coords_loc, 2, gl::FLOAT, gl::FALSE, 0, ptr::null());
        gl::EnableVertexAttribArray(tex_coords_loc);
        gl::BindBuffer(gl::ARRAY_BUFFER, normals_vbo);
        gl::VertexAttribPointer(normals_loc, 3, gl::FLOAT, gl::FALSE, 0, ptr::null());
        gl::EnableVertexAttribArray(normals_loc);
    }
    assert!(vao > 0);

    let points_handle = BufferHandle::new(points_vbo, vao);
    let tex_coords_handle = BufferHandle::new(tex_coords_vbo, vao);
    let normals_handle = BufferHandle::new(normals_vbo, vao);

    context.gl.buffers.insert(id, vec![points_handle, tex_coords_handle, normals_handle]);
    context.entities.model_matrices.insert(id, model_mat);
    context.entities.meshes.insert(id, mesh);
}

///
/// Load the triforce shader program.
///
fn create_triforce_shaders(context: &mut GameContext, id: EntityID) {
    let sp = glh::create_program_from_files(
        &context.gl,
        &context.shader_file("triangle.vert.glsl"),
        &context.shader_file("triangle.frag.glsl")
    ).unwrap();
    assert!(sp > 0);

    let sp_model_mat_loc = unsafe {
        gl::GetUniformLocation(sp, glh::gl_str("model_mat").as_ptr())
    };
    assert!(sp_model_mat_loc > -1);

    let sp_view_mat_loc = unsafe {
        gl::GetUniformLocation(sp, glh::gl_str("view_mat").as_ptr())
    };
    assert!(sp_view_mat_loc > -1);

    let sp_proj_mat_loc = unsafe {
        gl::GetUniformLocation(sp, glh::gl_str("proj_mat").as_ptr())
    };
    assert!(sp_proj_mat_loc > -1);

    let mut shader = ShaderProgram::new(ShaderProgramHandle::from(sp));
    shader.uniforms.insert(String::from("model_mat"), ShaderUniformHandle::from(sp_model_mat_loc));
    shader.uniforms.insert(String::from("view_mat"), ShaderUniformHandle::from(sp_view_mat_loc));
    shader.uniforms.insert(String::from("proj_mat"), ShaderUniformHandle::from(sp_proj_mat_loc));

    context.gl.shaders.insert(id, shader);
}

///
/// Load the triforce texture.
///
fn create_triforce_texture(context: &mut GameContext, id: EntityID) {
    let tex_image = texture::load_file(&context.asset_file("triangle.png")).unwrap();
    let tex = load_texture(&tex_image, gl::CLAMP_TO_EDGE).unwrap();

    context.entities.textures.insert(id, tex_image);
    context.gl.textures.insert(id, tex);
}

///
/// Send the uniform variables for a triforce to the GPU.
///
fn create_triforce_uniforms(context: &GameContext, id: EntityID) {
    let shader = &context.gl.shaders[&id];
    unsafe {
        gl::UseProgram(shader.handle.into());
        gl::UniformMatrix4fv(shader.uniforms["model_mat"].into(), 1, gl::FALSE, context.entities.model_matrices[&id].as_ptr());
        gl::UniformMatrix4fv(shader.uniforms["view_mat"].into(), 1, gl::FALSE, context.camera.view_mat.as_ptr());
        gl::UniformMatrix4fv(shader.uniforms["proj_mat"].into(), 1, gl::FALSE, context.camera.proj_mat.as_ptr());
    }
}

///
/// Reset the position of the camera to the default position and orientation.
///
fn reset_camera_to_default(context: &mut GameContext) {
    let width = context.gl.width as f32;
    let height = context.gl.height as f32;
    context.camera = create_camera(width, height);
}

///
/// The GLFW frame buffer size callback function. This is normally set using 
/// the GLFW `glfwSetFramebufferSizeCallback` function, but instead we explicitly
/// handle window resizing in our state updates on the application side. Run this function 
/// whenever the size of the viewport changes.
///
#[inline]
fn glfw_framebuffer_size_callback(context: &mut GameContext, width: u32, height: u32) {
    context.gl.width = width;
    context.gl.height = height;

    let aspect = context.gl.width as f32 / context.gl.height as f32;
    context.camera.aspect = aspect;
    context.camera.proj_mat = math::perspective((
        context.camera.fov, aspect, context.camera.near, context.camera.far
    ));
}

///
/// Initialize the demo.
///
fn init_game_state(ids: &[EntityID]) -> GameContext {
    let config = config::load(CONFIG_FILE).unwrap();
    let mut gl_state = match glh::start_gl(720, 480, &config.gl_log_file) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Failed to Initialize OpenGL context. Got error:");
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    let camera = create_camera(gl_state.width as f32, gl_state.height as f32);
    let light = create_light();
    let mut context = GameContext {
        config: config,
        gl: gl_state,
        camera: camera,
        light: light,
        entities: EntityDatabase::new(),
    };

    let model_mats = [
        Matrix4::from_scale(2.0) * Matrix4::from_rotation_z(180.0) * Matrix4::from_translation(math::vec3(( 0.0,       0.5, 2.0))),
        Matrix4::from_scale(2.0) * Matrix4::from_rotation_z(180.0) * Matrix4::from_translation(math::vec3((-0.577350, -0.5, 2.0))),
        Matrix4::from_scale(2.0) * Matrix4::from_rotation_z(180.0) * Matrix4::from_translation(math::vec3(( 0.577350, -0.5, 2.0))),
    ];

    create_ground_plane_shaders(&mut context, ids[0]);
    create_ground_plane_geometry(&mut context, ids[0]);
    create_ground_plane_uniforms(&context, ids[0]);
    create_ground_plane_texture(&mut context, ids[0]);
    create_triforce_shaders(&mut context, ids[1]);
    create_triforce_geometry(&mut context, ids[1], model_mats[0]);
    create_triforce_uniforms(&mut context, ids[1]);
    create_triforce_texture(&mut context, ids[1]);
    create_triforce_lights(&mut context, ids[1]);
    create_triforce_shaders(&mut context, ids[2]);
    create_triforce_geometry(&mut context, ids[2], model_mats[1]);
    create_triforce_uniforms(&mut context, ids[2]);
    create_triforce_texture(&mut context, ids[2]);
    create_triforce_lights(&mut context, ids[2]);
    create_triforce_shaders(&mut context, ids[3]);
    create_triforce_geometry(&mut context, ids[3], model_mats[2]);
    create_triforce_uniforms(&mut context, ids[3]);
    create_triforce_texture(&mut context, ids[3]);
    create_triforce_lights(&mut context, ids[3]);

    context
}

fn main() {
    let ids = [EntityID::new(0), EntityID::new(1), EntityID::new(2), EntityID::new(3)];
    let mut context = init_game_state(&ids);

    // Triforce animation parameters.
    let v_triforce: f32 = 5.0; // Meters per second.
    let mut vhat_triforce = math::vec3((1.0, 0.0, 0.0));
    let mut position_triforce = 0.0;
    let mut direction = 1.0;

    unsafe {
        // Enable depth testing.
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthFunc(gl::LESS);
        gl::Enable(gl::CULL_FACE);
        gl::CullFace(gl::BACK);
        gl::FrontFace(gl::CCW);
        // Gray background.
        gl::ClearColor(0.2, 0.2, 0.2, 1.0);
        gl::Viewport(0, 0, context.gl.width as i32, context.gl.height as i32);
    }

    /* --------------------------- GAME LOOP ------------------------------- */
    while !context.gl.window.should_close() {
        // Check input.
        let elapsed_seconds = glh::update_timers(&mut context.gl);

        // Update the game world.
        glh::update_fps_counter(&mut context.gl);

        context.gl.glfw.poll_events();

        // Camera control keys.
        let mut cam_moved = false;
        let mut move_to = math::vec3((0.0, 0.0, 0.0));
        let mut cam_yaw = 0.0;
        let mut cam_pitch = 0.0;
        let mut cam_roll = 0.0;
        match context.gl.window.get_key(Key::A) {
            Action::Press | Action::Repeat => {
                move_to.x -= context.camera.speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
            }
            _ => {}
        }
        match context.gl.window.get_key(Key::D) {
            Action::Press | Action::Repeat => {
                move_to.x += context.camera.speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
            }
            _ => {}
        }
        match context.gl.window.get_key(Key::Q) {
            Action::Press | Action::Repeat => {
                move_to.y += context.camera.speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
            }
            _ => {}
        }
        match context.gl.window.get_key(Key::E) {
            Action::Press | Action::Repeat => {
                move_to.y -= context.camera.speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
            }
            _ => {}
        }
        match context.gl.window.get_key(Key::W) {
            Action::Press | Action::Repeat => {
                move_to.z -= context.camera.speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
            }
            _ => {}
        }
        match context.gl.window.get_key(Key::S) {
            Action::Press | Action::Repeat => {
                move_to.z += context.camera.speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
            }
            _ => {}
        }
        match context.gl.window.get_key(Key::Left) {
            Action::Press | Action::Repeat => {
                cam_yaw += context.camera.yaw_speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
                let q_yaw = Quaternion::from_axis_deg(cam_yaw, math::vec3(context.camera.up));
                context.camera.axis = q_yaw * &context.camera.axis;
            }
            _ => {}
        }
        match context.gl.window.get_key(Key::Right) {
            Action::Press | Action::Repeat => {
                cam_yaw -= context.camera.yaw_speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
                let q_yaw = Quaternion::from_axis_deg(cam_yaw, math::vec3(context.camera.up));
                context.camera.axis = q_yaw * &context.camera.axis;
            }
            _ => {}
        }
        match context.gl.window.get_key(Key::Up) {
            Action::Press | Action::Repeat => {
                cam_pitch += context.camera.yaw_speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
                let q_pitch = Quaternion::from_axis_deg(cam_pitch, math::vec3(context.camera.rgt));
                context.camera.axis = q_pitch * &context.camera.axis;
            }
            _ => {}
        }
        match context.gl.window.get_key(Key::Down) {
            Action::Press | Action::Repeat => {
                cam_pitch -= context.camera.yaw_speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
                let q_pitch = Quaternion::from_axis_deg(cam_pitch, math::vec3(context.camera.rgt));
                context.camera.axis = q_pitch * &context.camera.axis;
            }
            _ => {}
        }
        match context.gl.window.get_key(Key::Z) {
            Action::Press | Action::Repeat => {
                cam_roll -= context.camera.yaw_speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
                let q_roll = Quaternion::from_axis_deg(cam_roll, math::vec3(context.camera.fwd));
                context.camera.axis = q_roll * &context.camera.axis;
            }
            _ => {}
        }
        match context.gl.window.get_key(Key::C) {
            Action::Press | Action::Repeat => {
                cam_roll += context.camera.yaw_speed * (elapsed_seconds as GLfloat);
                cam_moved = true;
                let q_roll = Quaternion::from_axis_deg(cam_roll, math::vec3(context.camera.fwd));
                context.camera.axis = q_roll * &context.camera.axis;
            }
            _ => {}
        }
        match context.gl.window.get_key(Key::Backspace) {
            Action::Press | Action::Repeat => {
                reset_camera_to_default(&mut context);
                cam_moved = true;
            }
            _ => {}
        }
        match context.gl.window.get_key(Key::Space) {
            Action::Press | Action::Repeat => {
                println!("axis = {}; norm = {}", context.camera.axis, context.camera.axis.norm());
            }
            _ => {}
        }
        match context.gl.window.get_key(Key::Escape) {
            Action::Press | Action::Repeat => {
                context.gl.window.set_should_close(true);
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

            let gp_sp = &context.gl.shaders[&ids[0]];
            let gp_view_mat_loc = gp_sp.uniforms["view_mat"];
            unsafe {
                gl::UseProgram(gp_sp.handle.into());
                gl::UniformMatrix4fv(gp_view_mat_loc.into(), 1, gl::FALSE, context.camera.view_mat.as_ptr());
            }

            let tri_sp1 = &context.gl.shaders[&ids[1]];
            let tri_sp_view_mat_loc1 = tri_sp1.uniforms["view_mat"];
            unsafe {
                gl::UseProgram(tri_sp1.handle.into());
                gl::UniformMatrix4fv(tri_sp_view_mat_loc1.into(), 1, gl::FALSE, context.camera.view_mat.as_ptr());
            }

            let tri_sp2 = &context.gl.shaders[&ids[2]];
            let tri_sp_view_mat_loc2 = tri_sp2.uniforms["view_mat"];
            unsafe {
                gl::UseProgram(tri_sp2.handle.into());
                gl::UniformMatrix4fv(tri_sp_view_mat_loc2.into(), 1, gl::FALSE, context.camera.view_mat.as_ptr());
            }

            let tri_sp3 = &context.gl.shaders[&ids[3]];
            let tri_sp_view_mat_loc3 = tri_sp3.uniforms["view_mat"];
            unsafe {
                gl::UseProgram(tri_sp3.handle.into());
                gl::UniformMatrix4fv(tri_sp_view_mat_loc3.into(), 1, gl::FALSE, context.camera.view_mat.as_ptr());
            }
        }

        let (width, height) = context.gl.window.get_framebuffer_size();
        if (width != context.gl.width as i32) && (height != context.gl.height as i32) {
            glfw_framebuffer_size_callback(&mut context, width as u32, height as u32);
        }

        // Update the kinematics of the triforce.
        let dx = v_triforce * elapsed_seconds as f32;
        position_triforce += dx * direction;
        if position_triforce > 10.0 || position_triforce < -10.0 {
            vhat_triforce = -vhat_triforce;
            direction = -direction;
        }
        let trans_mat = Matrix4::from_translation(vhat_triforce * dx);
        let model_mat = context.entities.model_matrices[&ids[1]];
        context.entities.model_matrices.insert(ids[1], trans_mat * model_mat);
        let model_mat = context.entities.model_matrices[&ids[2]];
        context.entities.model_matrices.insert(ids[2], trans_mat * model_mat);
        let model_mat = context.entities.model_matrices[&ids[3]];
        context.entities.model_matrices.insert(ids[3], trans_mat * model_mat);

        let tri_sp1 = &context.gl.shaders[&ids[1]];
        let tri_sp_model_mat_loc1 = tri_sp1.uniforms["model_mat"];
        unsafe {
            gl::UseProgram(tri_sp1.handle.into());
            gl::UniformMatrix4fv(
                tri_sp_model_mat_loc1.into(), 1, gl::FALSE,
                context.entities.model_matrices[&ids[1]].as_ptr()
            );
        }

        let tri_sp2 = &context.gl.shaders[&ids[2]];
        let tri_sp_model_mat_loc2 = tri_sp1.uniforms["model_mat"];
        unsafe {
            gl::UseProgram(tri_sp2.handle.into());
            gl::UniformMatrix4fv(
                tri_sp_model_mat_loc2.into(), 1, gl::FALSE,
                context.entities.model_matrices[&ids[2]].as_ptr()
            );
        }

        let tri_sp3 = &context.gl.shaders[&ids[3]];
        let tri_sp_model_mat_loc3 = tri_sp1.uniforms["model_mat"];
        unsafe {
            gl::UseProgram(tri_sp3.handle.into());
            gl::UniformMatrix4fv(
                tri_sp_model_mat_loc3.into(), 1, gl::FALSE,
                context.entities.model_matrices[&ids[3]].as_ptr()
            );
        }

        // Render the results.
        unsafe {
            // Clear the screen.
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            gl::ClearColor(0.2, 0.2, 0.2, 1.0);
            gl::Viewport(0, 0, context.gl.width as i32, context.gl.height as i32);

            // Render the ground plane.
            gl::UseProgram(context.gl.shaders[&ids[0]].handle.into());
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, context.gl.textures[&ids[0]].into());
            gl::BindVertexArray(context.gl.buffers[&ids[0]][0].vao);
            gl::DrawArrays(gl::TRIANGLES, 0, context.entities.meshes[&ids[0]].len() as i32);

            // Render the triforce.
            gl::UseProgram(context.gl.shaders[&ids[1]].handle.into());
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, context.gl.textures[&ids[1]].into());
            gl::BindVertexArray(context.gl.buffers[&ids[1]][0].vao);
            gl::DrawArrays(gl::TRIANGLES, 0, context.entities.meshes[&ids[1]].len() as i32);

            gl::UseProgram(context.gl.shaders[&ids[2]].handle.into());
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, context.gl.textures[&ids[2]].into());
            gl::BindVertexArray(context.gl.buffers[&ids[2]][0].vao);
            gl::DrawArrays(gl::TRIANGLES, 0, context.entities.meshes[&ids[2]].len() as i32);

            gl::UseProgram(context.gl.shaders[&ids[3]].handle.into());
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, context.gl.textures[&ids[3]].into());
            gl::BindVertexArray(context.gl.buffers[&ids[3]][0].vao);
            gl::DrawArrays(gl::TRIANGLES, 0, context.entities.meshes[&ids[3]].len() as i32);
        }
        
        // Send the results to the output.
        context.gl.window.swap_buffers();
    }
}
