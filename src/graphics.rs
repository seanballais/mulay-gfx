// Based on:
// http://nercury.github.io/rust/opengl/tutorial/2018/02/10
//       /opengl-in-rust-from-scratch-03-compiling-shaders.html
extern crate gl;

use crate::assets::Shader;

use std::error::Error;
use std::fmt;
use std::mem;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProgramErrorKind {
    ShaderAssetPoisoned,
}

#[derive(Debug)]
pub struct ProgramError {
    source: Option<Box<dyn Error + 'static>>,
    message: String,
    kind: ProgramErrorKind,
}

impl ProgramError {
    pub fn new(
        message: impl AsRef<str>,
        kind: ProgramErrorKind,
        source: Option<Box<dyn Error + 'static>>,
    ) -> ProgramError {
        ProgramError {
            source,
            message: message.as_ref().into(),
            kind,
        }
    }
}

impl fmt::Display for ProgramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for ProgramError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_deref()
    }
}

pub struct Program {
    id: gl::types::GLuint,
    shaders: Vec<Arc<Mutex<Shader>>>,
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.id) }
    }
}

impl Program {
    pub fn new(shaders: Vec<Arc<Mutex<Shader>>>) -> Result<Self, ProgramError> {
        let program_id: gl::types::GLuint = unsafe { gl::CreateProgram() };
        let program: Self = Self {
            id: program_id,
            shaders: shaders,
        };

        for shader in &program.shaders {
            match shader.lock() {
                Ok(shader_ptr) => {
                    unsafe {
                        gl::AttachShader(program_id, shader_ptr.get_shader_id());
                    };
                }
                Err(_) => {
                    return Err(ProgramError::new(
                        "shader asset is poisoned",
                        ProgramErrorKind::ShaderAssetPoisoned,
                        None,
                    ))
                }
            }
        }

        unsafe {
            gl::LinkProgram(program_id);
        };

        for shader in &program.shaders {
            match shader.lock() {
                Ok(shader_ptr) => {
                    unsafe {
                        gl::DetachShader(program_id, shader_ptr.get_shader_id());
                    };
                }
                Err(_) => {
                    return Err(ProgramError::new(
                        "shader asset is poisoned",
                        ProgramErrorKind::ShaderAssetPoisoned,
                        None,
                    ))
                }
            }
        }

        Ok(program)
    }

    pub fn id(&self) -> gl::types::GLuint {
        self.id
    }

    pub fn reload(&mut self) -> Result<(), ProgramError> {
        unsafe {
            gl::DeleteProgram(self.id);
        };

        let program_id: gl::types::GLuint = unsafe { gl::CreateProgram() };
        for shader in &self.shaders {
            match shader.lock() {
                Ok(shader_ptr) => {
                    unsafe {
                        gl::AttachShader(program_id, shader_ptr.get_shader_id());
                    };
                }
                Err(_) => {
                    return Err(ProgramError::new(
                        "shader asset is poisoned",
                        ProgramErrorKind::ShaderAssetPoisoned,
                        None,
                    ))
                }
            }
        }

        unsafe {
            gl::LinkProgram(program_id);
        };

        for shader in &self.shaders {
            match shader.lock() {
                Ok(shader_ptr) => {
                    unsafe {
                        gl::DetachShader(program_id, shader_ptr.get_shader_id());
                    };
                }
                Err(_) => {
                    return Err(ProgramError::new(
                        "shader asset is poisoned",
                        ProgramErrorKind::ShaderAssetPoisoned,
                        None,
                    ))
                }
            }
        }

        self.id = program_id;

        Ok(())
    }
}
