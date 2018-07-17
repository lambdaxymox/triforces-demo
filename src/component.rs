use std::collections::HashMap;
use obj::ObjMesh;
use math::Matrix4;


pub struct ShaderSource {
    src : String,
    uniforms: Vec<String>,
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct ShaderUniformHandle {
    inner: i32,
}

impl From<i32> for ShaderUniformHandle {
    #[inline]
    fn from(handle: i32) -> ShaderUniformHandle {
        ShaderUniformHandle {
            inner: handle,
        }
    }
}

impl Into<i32> for ShaderUniformHandle {
    #[inline]
    fn into(self) -> i32 {
        self.inner
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct ShaderProgramHandle {
    inner: u32,
}

impl From<u32> for ShaderProgramHandle {
    #[inline]
    fn from(handle: u32) -> ShaderProgramHandle {
        ShaderProgramHandle {
            inner: handle,
        }
    }
}

impl Into<u32> for ShaderProgramHandle {
    #[inline]
    fn into(self) -> u32 {
        self.inner
    }
}

pub struct ShaderProgram {
    pub handle: ShaderProgramHandle,
    pub uniforms: HashMap<String, ShaderUniformHandle>,
}

impl ShaderProgram {
    #[inline]
    pub fn new(handle: ShaderProgramHandle) -> ShaderProgram {
        ShaderProgram {
            handle: handle,
            uniforms: HashMap::new(),
        }
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub struct BufferHandle {
    pub vbo: u32,
    pub vao: u32,
}

impl BufferHandle {
    #[inline]
    pub fn new(vbo: u32, vao: u32) -> BufferHandle {
        BufferHandle { vbo, vao }
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub struct EntityID {
    id: u32,
}

impl EntityID {
    #[inline]
    pub fn new(id: u32) -> EntityID {
        EntityID { id }
    }
}
