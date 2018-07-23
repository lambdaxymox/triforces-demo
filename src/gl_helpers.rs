use gl;
use gl::types::{GLchar, GLenum, GLint, GLubyte, GLuint};
use glfw;
use glfw::{Context};

use std::ffi::CStr;
use std::fs::File;
use std::io::{Read, BufReader};
use std::sync::mpsc::Receiver;
use std::ptr;
use std::fmt;
use std::collections::HashMap;

use logger::Logger;
use component::{ShaderProgram, ShaderProgramHandle, BufferHandle, EntityID, TextureHandle};


// 256 Kilobytes.
const MAX_SHADER_LENGTH: usize = 262144;


#[inline]
pub fn glubyte_ptr_to_string(cstr: *const GLubyte) -> String {
    unsafe {
        CStr::from_ptr(cstr as *const i8).to_string_lossy().into_owned()
    }
}

///
/// A record containing a description of the GL capabilities on a local machine.
/// The contents of this record can be used for debugging OpenGL problems on
/// different machines.
/// 
struct GLParameters {
    params: Vec<(String, String)>
}

impl fmt::Display for GLParameters {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "GL Context Params:").unwrap();
        for &(ref param, ref value) in self.params.iter() {
            writeln!(f, "{} = {}", param, value).unwrap();
        }
        writeln!(f)
    }
}

///
/// Print out the GL capabilities on a local machine. This is handy for debugging
/// OpenGL program problems on other people's machines.
///
fn gl_params() -> GLParameters {
    let params: [GLenum; 12] = [
        gl::MAX_COMBINED_TEXTURE_IMAGE_UNITS,
        gl::MAX_CUBE_MAP_TEXTURE_SIZE,
        gl::MAX_DRAW_BUFFERS,
        gl::MAX_FRAGMENT_UNIFORM_COMPONENTS,
        gl::MAX_TEXTURE_IMAGE_UNITS,
        gl::MAX_TEXTURE_SIZE,
        gl::MAX_VARYING_FLOATS,
        gl::MAX_VERTEX_ATTRIBS,
        gl::MAX_VERTEX_TEXTURE_IMAGE_UNITS,
        gl::MAX_VERTEX_UNIFORM_COMPONENTS,
        gl::MAX_VIEWPORT_DIMS,
        gl::STEREO,
    ];
    let names: [&str; 12] = [
        "GL_MAX_COMBINED_TEXTURE_IMAGE_UNITS",
        "GL_MAX_CUBE_MAP_TEXTURE_SIZE",
        "GL_MAX_DRAW_BUFFERS",
        "GL_MAX_FRAGMENT_UNIFORM_COMPONENTS",
        "GL_MAX_TEXTURE_IMAGE_UNITS",
        "GL_MAX_TEXTURE_SIZE",
        "GL_MAX_VARYING_FLOATS",
        "GL_MAX_VERTEX_ATTRIBS",
        "GL_MAX_VERTEX_TEXTURE_IMAGE_UNITS",
        "GL_MAX_VERTEX_UNIFORM_COMPONENTS",
        "GL_MAX_VIEWPORT_DIMS",
        "GL_STEREO",
    ];
    let mut vec: Vec<(String, String)> = vec![];
    // Integers: this only works if the order is 0-10 integer return types.
    for i in 0..10 {
        let mut v = 0;
        unsafe { 
            gl::GetIntegerv(params[i], &mut v);
        }
        vec.push((format!("{}", names[i]), format!("{}", v)));
    }
    // others
    let mut v: [GLint; 2] = [0; 2];
    unsafe {    
        gl::GetIntegerv(params[10], &mut v[0]);
    }
    vec.push((format!("{}", names[10]), format!("{} {}", v[0], v[1])));
    let mut s = 0;
    unsafe {
        gl::GetBooleanv(params[11], &mut s);
    }
    vec.push((format!("{}", names[11]), format!("{}", s as usize)));

    GLParameters {
        params: vec,
    }
}

///
/// A record for storing all the OpenGL state needed on the application side
/// of the graphics application in order to manage OpenGL and GLFW.
///
pub struct GLState {
    pub glfw: glfw::Glfw,
    pub window: glfw::Window,
    pub events: Receiver<(f64, glfw::WindowEvent)>,
    pub logger: Logger,
    pub width: u32,
    pub height: u32,
    pub channel_depth: u32,
    pub running_time_seconds: f64,
    pub framerate_time_seconds: f64,
    pub frame_count: u32,
    pub shaders: HashMap<EntityID, ShaderProgram>,
    pub textures: HashMap<EntityID, TextureHandle>,
    pub buffers: HashMap<EntityID, Vec<BufferHandle>>,
}

///
/// Initialize a new OpenGL context and start a new GLFW window.
///
#[cfg(target_os = "macos")]
pub fn start_gl(width: u32, height: u32, log_file: &str) -> Result<GLState, String> {
    // Initiate a logger.
    let logger = Logger::from(log_file);
    logger.restart();

    // Start GL context and O/S window using the GLFW helper library.
    log!(logger, "Starting GLFW");
    log!(logger, "Using GLFW version {}", glfw::get_version_string());

    // Start a GL context and OS window using the GLFW helper library.
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    glfw.window_hint(glfw::WindowHint::Samples(Some(4)));

    /* -------------------------------- APPLE --------------------------- */
    glfw.window_hint(glfw::WindowHint::ContextVersionMajor(3));
    glfw.window_hint(glfw::WindowHint::ContextVersionMinor(2));
    glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));
    /* -------------------------------------------------------------------*/

    log!(logger, "Started GLFW successfully");
    let maybe_glfw_window = glfw.create_window(
        width, height, &format!("Triforces DEMO @ {:.2} FPS", 0.0), glfw::WindowMode::Windowed
    );
    let (mut window, events) = match maybe_glfw_window {
        Some(tuple) => tuple,
        None => {
            log!(logger, "Failed to create GLFW window");
            return Err(format!("Failed to create GLFW window."));
        }
    };

    window.make_current();
    window.set_key_polling(true);
    window.set_size_polling(true);
    window.set_refresh_polling(true);
    window.set_size_polling(true);

    // Load the OpenGl function pointers.
    gl::load_with(|symbol| { window.get_proc_address(symbol) as *const _ });

    // Get renderer and version information.
    let renderer = glubyte_ptr_to_string(unsafe { gl::GetString(gl::RENDERER) });
    println!("Renderer: {}", renderer);
    log!(logger, "Renderer: {}", renderer);

    let version = glubyte_ptr_to_string(unsafe { gl::GetString(gl::VERSION) });
    println!("OpenGL version supported: {}", version);
    log!(logger, "OpenGL version supported: {}", version);
    log!(logger, "{}", gl_params());

    Ok(GLState {
        glfw: glfw,
        window: window,
        events: events,
        logger: logger,
        width: width,
        height: height,
        channel_depth: 3,
        running_time_seconds: 0.0,
        framerate_time_seconds: 0.0,
        frame_count: 0,
        shaders: HashMap::new(),
        textures: HashMap::new(),
        buffers: HashMap::new(),
    })
}

///
/// Initialize a new OpenGL context and start a new GLFW window. 
///
#[cfg(not(target_os = "macos"))]
pub fn start_gl(width: u32, height: u32, log_file: &str) -> Result<GLState, String> {
    // Initiate a logger.
    let logger = Logger::from(log_file);
    logger.restart();

    // Start GL context and O/S window using the GLFW helper library.
    log!(logger, "Starting GLFW");
    log!(logger, "Using GLFW version {}", glfw::get_version_string());

    // Start a GL context and OS window using the GLFW helper library.
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    glfw.window_hint(glfw::WindowHint::Samples(Some(4)));

    log!(logger, "Started GLFW successfully");
    let maybe_glfw_window = glfw.create_window(
        width, height, &format!("Metroid DEMO @ {:.2} FPS", 0.0), glfw::WindowMode::Windowed
    );
    let (mut window, events) = match maybe_glfw_window {
        Some(tuple) => tuple,
        None => {
            log!(logger, "Failed to create GLFW window");
            return Err(format!("Failed to create GLFW window."));
        }
    };

    window.make_current();
    window.set_key_polling(true);
    window.set_size_polling(true);
    window.set_refresh_polling(true);
    window.set_size_polling(true);

    // Load the OpenGl function pointers.
    gl::load_with(|symbol| { window.get_proc_address(symbol) as *const _ });

    // Get renderer and version information.
    let renderer = glubyte_ptr_to_string(unsafe { gl::GetString(gl::RENDERER) });
    println!("Renderer: {}", renderer);
    log!(logger, "Renderer: {}", renderer);

    let version = glubyte_ptr_to_string(unsafe { gl::GetString(gl::VERSION) });
    println!("OpenGL version supported: {}", version);
    log!(logger, "OpenGL version supported: {}", version);
    log!(logger, "{}", gl_params());

    Ok(GLState {
        glfw: glfw, 
        window: window, 
        events: events,
        logger: logger,
        width: width,
        height: height,
        channel_depth: 3,
        running_time_seconds: 0.0,
        framerate_time_seconds: 0.0,
        frame_count: 0,
        shaders: HashMap::new(),
        textures: HashMap::new(),
        buffers: HashMap::new(),
    })
}

///
/// Updates the timers in a GL context. It returns the elapsed time since the last call to
/// `update_timers`.
///
#[inline]
pub fn update_timers(context: &mut GLState) -> f64 {
    let current_seconds = context.glfw.get_time();
    let elapsed_seconds = current_seconds - context.running_time_seconds;
    context.running_time_seconds = current_seconds;

    elapsed_seconds
}

///
/// Update the framerate and display in the window titlebar.
///
#[inline]
pub fn update_fps_counter(context: &mut GLState) {     
    let current_time_seconds = context.glfw.get_time();
    let elapsed_seconds = current_time_seconds - context.framerate_time_seconds;
    if elapsed_seconds > 0.5 {
        context.framerate_time_seconds = current_time_seconds;
        let fps = context.frame_count as f64 / elapsed_seconds;
        context.window.set_title(&format!("Metroid DEMO @ {:.2} FPS", fps));
        context.frame_count = 0;
    }

    context.frame_count += 1;
}


pub fn parse_shader(file_name: &str, shader_str: &mut [u8]) -> Result<usize, String> {
    shader_str[0] = 0;
    let file = match File::open(file_name) {
        Ok(val) => val,
        Err(_) => {
            return Err(format!("ERROR: opening file for reading: {}\n", file_name));
        }
    };

    let mut reader = BufReader::new(file);
    let bytes_read = match reader.read(shader_str) {
        Ok(val) => val,
        Err(_) => {
            return Err(format!("ERROR: reading shader file {}\n", file_name));
        }
    };

    // Append \0 character to end of the shader string to mark the end of a C string.
    shader_str[bytes_read] = 0;

    Ok(bytes_read)
}

///
/// A record containing all the relevant compilation log information for a
/// given GLSL shader compiled at run time.
///
pub struct ShaderLog {
    index: GLuint,
    log: String,
}

impl fmt::Display for ShaderLog {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Shader info log for GL index {}:", self.index).unwrap();
        writeln!(f, "{}", self.log)
    }
}

///
/// Query the shader information log generated during shader compilation from
/// OpenGL.
/// 
pub fn shader_info_log(shader_index: GLuint) -> ShaderLog {
    let mut actual_length = 0;
    let mut raw_log: [i8; 2048] = [0; 2048];
    unsafe {
        gl::GetShaderInfoLog(shader_index, raw_log.len() as i32, &mut actual_length, &mut raw_log[0]);
    }
    
    let mut log = String::new();
    for i in 0..actual_length as usize {
        log.push(raw_log[i] as u8 as char);
    }

    ShaderLog { index: shader_index, log: log }
}

pub fn create_shader(context: &GLState, file_name: &str, gl_type: GLenum) -> Result<GLuint, String> {
    log!(context.logger, "Creating shader from {}...\n", file_name);

    let mut shader_string = vec![0; MAX_SHADER_LENGTH];
    let bytes_read = match parse_shader(file_name, &mut shader_string) {
        Ok(val) => val,
        Err(st) => {
            log_err!(context.logger, &st);
            return Err(st);
        }
    };

    if bytes_read >= (MAX_SHADER_LENGTH - 1) {
        log!(context.logger,
            "WARNING: The shader was truncated because the shader code 
            was longer than MAX_SHADER_LENGTH {} bytes.", MAX_SHADER_LENGTH
        );
    }

    let shader = unsafe { gl::CreateShader(gl_type) };
    let p = shader_string.as_ptr() as *const GLchar;
    unsafe {
        gl::ShaderSource(shader, 1, &p, ptr::null());
        gl::CompileShader(shader);
    }

    // Check for shader compile errors.
    let mut params = -1;
    unsafe {
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut params);
    }

    if params != gl::TRUE as i32 {
        let log = shader_info_log(shader);
        log_err!(context.logger, "ERROR: GL shader index {} did not compile\n{}", shader, log);
        return Err(format!("{}", log));
    }
    log!(context.logger, "Shader compiled with index {}\n", shader);
    
    Ok(shader)
}


///
/// A record containing all the relevant compilation log information for a
/// given GLSL shader program compiled at run time.
///
pub struct ProgramLog {
    index: GLuint,
    log: String,
}

impl fmt::Display for ProgramLog {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Program info log for GL index {}:", self.index).unwrap();
        writeln!(f, "{}", self.log)
    }
}

///
/// Query the shader program information log generated during shader compilation 
/// from OpenGL.
/// 
pub fn program_info_log(index: GLuint) -> ProgramLog {
    let mut actual_length = 0;
    let mut raw_log = [0 as i8; 2048];
    unsafe {
        gl::GetProgramInfoLog(index, raw_log.len() as i32, &mut actual_length, &mut raw_log[0]);
    }
    
    let mut log = String::new();
    for i in 0..actual_length as usize {
        log.push(raw_log[i] as u8 as char);
    }

    ProgramLog { index: index, log: log }
}

///
/// Validate a shader program.
///
pub fn is_program_valid(logger: &Logger, sp: GLuint) -> bool {
    let mut params = -1;
    unsafe {
        gl::ValidateProgram(sp);
        gl::GetProgramiv(sp, gl::VALIDATE_STATUS, &mut params);
    }

    if params != gl::TRUE as i32 {
        log_err!(logger, "Program {} GL_VALIDATE_STATUS = GL_FALSE\n", sp);
        log_err!(logger, "{}", program_info_log(sp));
        
        return false;
    }

    log!(logger, "Program {} GL_VALIDATE_STATUS = {}\n", sp, params);
    
    true
}

///
/// Compile and link a shader program.
///
pub fn create_program(context: &GLState, vertex_shader: GLuint, fragment_shader: GLuint) -> Result<GLuint, String> {
    unsafe {
        let program = gl::CreateProgram();
        log!(context.logger, "Created programme {}. attaching shaders {} and {}...\n", 
            program, vertex_shader, fragment_shader
        );
        gl::AttachShader(program, vertex_shader);
        gl::AttachShader(program, fragment_shader);

        // Link the shader program. If binding input attributes do that before linking.
        gl::LinkProgram(program);
        
        let mut params = -1;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut params);
        if params != gl::TRUE as i32 {
            log_err!(context.logger, "ERROR: could not link shader programme GL index {}\n", program);
            log_err!(context.logger, "{}", program_info_log(program));
        
            return Err(String::new());
        }
        is_program_valid(&context.logger, program);
        // Delete shaders here to free memory.
        gl::DeleteShader(vertex_shader);
        gl::DeleteShader(fragment_shader);
        
        Ok(program)
    }
}

///
/// Compile and link a shader program directly from the files.
///
pub fn create_program_from_files(context: &GLState, vert_file_name: &str, frag_file_name: &str) -> GLuint {
    let vertex_shader = create_shader(context, vert_file_name, gl::VERTEX_SHADER).unwrap();
    let fragment_shader = create_shader(context, frag_file_name, gl::FRAGMENT_SHADER).unwrap();
    let program = create_program(context, vertex_shader, fragment_shader).unwrap();

    program
}
