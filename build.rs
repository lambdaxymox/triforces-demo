extern crate gl_generator;
extern crate mini_obj;

use gl_generator::{Registry, Api, Profile, Fallbacks, GlobalGenerator};
use mini_obj as obj;

use std::env;
use std::io;
use std::io::Write;
use std::fs::File;
use std::path::Path;


fn generate_code_fragment<P: AsRef<Path>>(path: P) -> String {
    let model = obj::load_file(path).unwrap();
    let fragment = obj::to_rust_code(&model);

    fragment
}

fn write_code_fragment(fragment: &str, fragment_name: &str) -> io::Result<()> {
    let path = Path::new(".").join(fragment_name);
    let mut file = File::create(&path)?;
    file.write_all(fragment.as_bytes())?;
    file.sync_all()
}


#[cfg(target_os = "macos")]
fn register_gl_api(file: &mut File) {
    Registry::new(Api::Gl, (3, 3), Profile::Core, Fallbacks::All, [])
        .write_bindings(GlobalGenerator, file)
        .unwrap();
}

#[cfg(target_os = "windows")]
fn register_gl_api(file: &mut File) {
    Registry::new(Api::Gl, (3, 3), Profile::Core, Fallbacks::All, [])
        .write_bindings(GlobalGenerator, file)
        .unwrap();
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn register_gl_api(file: &mut File) {
    Registry::new(Api::Gl, (4, 6), Profile::Core, Fallbacks::All, [])
        .write_bindings(GlobalGenerator, file)
        .unwrap();
}

fn main() {
    let triangle = generate_code_fragment("assets/triangle.obj");
    write_code_fragment(&triangle, "triangle.obj.in").unwrap();

    let ground_plane = generate_code_fragment("assets/ground_plane.obj");
    write_code_fragment(&ground_plane, "ground_plane.obj.in").unwrap();

    let dest = env::var("OUT_DIR").unwrap();
    let mut file = File::create(&Path::new(&dest).join("gl_bindings.rs")).unwrap();

    register_gl_api(&mut file);
}
