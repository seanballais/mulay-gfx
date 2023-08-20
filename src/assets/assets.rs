extern crate gl;

use std::error::Error;
use std::ffi::{CString, OsStr};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::ptr;

use crate::c_bridge;

// Errors based on the implementation of std::io::Error and
// std::io::ErrorKind.

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AssetErrorKind {
    LoadingFailed,
    NotLoaded,
    Poisoned,
    InvalidFileExtension,
    ReloadingFailed,
}

#[derive(Debug)]
pub struct AssetError {
    source: Option<Box<dyn Error + 'static>>,
    message: String,
    kind: AssetErrorKind,
}

impl AssetError {
    pub fn new(
        message: impl AsRef<str>,
        kind: AssetErrorKind,
        source: Option<Box<dyn Error + 'static>>,
    ) -> AssetError {
        AssetError {
            source,
            message: message.as_ref().into(),
            kind,
        }
    }
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for AssetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_deref()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShaderErrorKind {
    MalformedSource,
    CompilationError,
}

#[derive(Debug)]
pub struct ShaderError {
    source: Option<Box<dyn Error + 'static>>,
    message: String,
    kind: ShaderErrorKind,
}

impl ShaderError {
    pub fn new(
        message: impl AsRef<str>,
        kind: ShaderErrorKind,
        source: Option<Box<dyn Error + 'static>>,
    ) -> ShaderError {
        ShaderError {
            source,
            message: message.as_ref().into(),
            kind,
        }
    }
}

impl fmt::Display for ShaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for ShaderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_deref()
    }
}

pub trait Asset {
    fn new<S: AsRef<str>>(id: S, file_path: &Path) -> Result<Self, AssetError>
    where
        Self: Sized;
    fn reload(&mut self) -> Result<(), AssetError>;
    fn destroy(&mut self) -> Result<(), AssetError>;
    fn is_loaded(&self) -> bool;
    fn get_src_file_path(&self) -> &Path;
}

pub struct Shader {
    id: String,
    shader_id: gl::types::GLuint,
    kind: gl::types::GLenum,
    src_file_path: PathBuf,
    is_loaded: bool,
    is_stale: bool,
}

impl Asset for Shader {
    fn new<S: AsRef<str>>(id: S, file_path: &Path) -> Result<Self, AssetError> {
        let file_ext: &OsStr = match file_path.extension() {
            Some(extension) => extension,
            None => {
                return Err(AssetError::new(
                    format!(
                        "shader source file from {} does not have a valid file extension",
                        file_path.to_string_lossy()
                    ),
                    AssetErrorKind::InvalidFileExtension,
                    None,
                ));
            }
        };

        let kind: gl::types::GLenum = match file_ext.to_str() {
            Some("vert") => gl::VERTEX_SHADER,
            Some("frag") => gl::FRAGMENT_SHADER,
            _ => {
                return Err(AssetError::new(
                    format!(
                        "shader source file extension of {} is neither \".vert\" or \".frag\".",
                        file_path.to_string_lossy()
                    ),
                    AssetErrorKind::InvalidFileExtension,
                    None,
                ));
            }
        };

        match fs::read_to_string(file_path) {
            Ok(contents) => {
                let shader_id: gl::types::GLuint = match Self::compile(contents.as_str(), kind) {
                    Ok(id) => id,
                    Err(error) => {
                        return Err(AssetError::new(
                            format!(
                                "unable to compile shader from {}",
                                file_path.to_string_lossy()
                            ),
                            AssetErrorKind::LoadingFailed,
                            Some(Box::new(error)),
                        ))
                    }
                };

                let shader: Self = Self {
                    id: id.as_ref().into(),
                    shader_id: shader_id,
                    kind: kind,
                    src_file_path: file_path.to_path_buf(),
                    is_loaded: true,
                    is_stale: false,
                };

                Ok(shader)
            }
            Err(error) => Err(AssetError::new(
                format!("unable to load asset from {}", file_path.to_string_lossy()),
                AssetErrorKind::LoadingFailed,
                Some(Box::new(error)),
            )),
        }
    }

    fn reload(&mut self) -> Result<(), AssetError> {
        if !self.is_loaded {
            return Err(AssetError::new(
                format!("asset, '{}', not yet loaded", self.id.as_str()),
                AssetErrorKind::NotLoaded,
                None,
            ));
        }

        match fs::read_to_string(self.src_file_path.as_path()) {
            Ok(contents) => {
                let new_shader_id: gl::types::GLuint =
                    match Self::compile(contents.as_str(), self.kind) {
                        Ok(id) => id,
                        Err(error) => {
                            return Err(AssetError::new(
                                format!(
                                    "unable to hot-reload shader from {}",
                                    self.src_file_path.to_string_lossy()
                                ),
                                AssetErrorKind::ReloadingFailed,
                                Some(Box::new(error)),
                            ))
                        }
                    };

                unsafe {
                    gl::DeleteShader(self.shader_id);
                }

                self.shader_id = new_shader_id;
                self.is_stale = false;
                Ok(())
            }
            Err(error) => Err(AssetError::new(
                format!("unable to reload asset, '{}'", self.id.as_str()),
                AssetErrorKind::LoadingFailed,
                Some(Box::new(error)),
            )),
        }
    }

    fn destroy(&mut self) -> Result<(), AssetError> {
        unsafe {
            gl::DeleteShader(self.shader_id);
        };

        self.id.clear();
        self.src_file_path.clear();
        self.is_loaded = false;

        Ok(())
    }

    fn is_loaded(&self) -> bool {
        self.is_loaded
    }

    fn get_src_file_path(&self) -> &Path {
        self.src_file_path.as_path()
    }
}

impl Shader {
    pub fn get_shader_id(&self) -> gl::types::GLuint {
        self.shader_id
    }

    pub fn get_shader_kind(&self) -> gl::types::GLenum {
        self.kind
    }

    // Based on:
    // http://nercury.github.io/rust/opengl/tutorial/2018/02/10
    //       /opengl-in-rust-from-scratch-03-compiling-shaders.html
    fn compile(src: &str, kind: gl::types::GLenum) -> Result<gl::types::GLuint, ShaderError> {
        let converted_src: CString = match CString::new(src) {
            Ok(src) => src,
            Err(error) => {
                return Err(ShaderError::new(
                    "malformed shader source",
                    ShaderErrorKind::MalformedSource,
                    Some(Box::new(error)),
                ));
            }
        };

        let shader_id: gl::types::GLuint = unsafe { gl::CreateShader(kind) };
        unsafe {
            gl::ShaderSource(
                shader_id,
                1,
                &converted_src.as_c_str().as_ptr(),
                ptr::null(),
            );
            gl::CompileShader(shader_id);
        };

        let mut success: gl::types::GLint = 1;
        unsafe {
            gl::GetShaderiv(shader_id, gl::COMPILE_STATUS, &mut success);
        };

        if success == 0 {
            let mut error_msg_length: gl::types::GLint = 0;
            unsafe {
                gl::GetShaderiv(shader_id, gl::INFO_LOG_LENGTH, &mut error_msg_length);
            }

            let error_msg: CString = c_bridge::create_sized_cstring(error_msg_length as usize);
            unsafe {
                gl::GetShaderInfoLog(
                    shader_id,
                    error_msg_length,
                    ptr::null_mut(),
                    error_msg.as_ptr() as *mut gl::types::GLchar,
                );
            };

            return Err(ShaderError::new(
                error_msg.to_string_lossy().into_owned(),
                ShaderErrorKind::CompilationError,
                None,
            ));
        }

        Ok(shader_id)
    }
}
