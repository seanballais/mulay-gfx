extern crate gl;
extern crate sdl2;

mod assets;
mod c_bridge;
mod graphics;

use graphics::Program;

use std::mem;
use std::os;
use std::sync::Arc;
use std::ptr;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::GLProfile;

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(GLProfile::Core);
    gl_attr.set_context_version(3, 3);

    let window = video_subsystem
        .window("MulayGFX", 640, 480)
        .opengl()
        .build()
        .unwrap();

    let _ctx = window.gl_create_context().unwrap();

    gl::load_with(|name| video_subsystem.gl_get_proc_address(name) as *const os::raw::c_void);

    debug_assert_eq!(gl_attr.context_profile(), GLProfile::Core);
    debug_assert_eq!(gl_attr.context_version(), (3, 3));

    // Set up data.
    let vertices = vec![
        -0.5f32, -0.5f32, 0.0f32, 0.5f32, -0.5f32, 0.0f32, 0.0f32, 0.5f32, 0.0f32,
    ];

    let mut shader_asset_manager = assets::AssetManager::<assets::Shader>::new();
    let vertex_shader = match shader_asset_manager.load_asset("vertex-shader", "assets/shaders/triangle.vert") {
        Ok(ptr) => ptr,
        Err(err) => panic!("{:?}", err), // For now. Maybe.
    };
    let fragment_shader = match shader_asset_manager.load_asset("fragment-shader", "assets/shaders/triangle.frag") {
        Ok(ptr) => ptr,
        Err(err) => panic!("{:?}", err), // For now. Maybe.
    };

    let shader_program: Program = match Program::new(vec![Arc::clone(&vertex_shader), Arc::clone(&fragment_shader)]) {
        Ok(program) => program,
        Err(err) => panic!("{:?}", err)
    };

    let mut vao_id: u32 = 0;
    let mut vbo_id: u32 = 0;
    unsafe {
        gl::GenVertexArrays(1, &mut vao_id);
        gl::GenBuffers(1, &mut vbo_id);

        gl::BindVertexArray(vao_id);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo_id);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (vertices.len() * mem::size_of::<f32>()) as gl::types::GLsizeiptr,
            vertices.as_ptr() as *const gl::types::GLvoid,
            gl::STATIC_DRAW,
        );
        gl::VertexAttribPointer(
            0,
            3,
            gl::FLOAT,
            gl::FALSE,
            (mem::size_of::<f32>() * 3) as i32,
            ptr::null(),
        );
        gl::EnableVertexAttribArray(0);
    }

    // Event Process
    let mut event_pump = sdl_context.event_pump().unwrap();

    loop {
        let mut do_quit = false;

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    do_quit = true;
                }
                _ => {}
            }
        }

        if do_quit {
            break;
        }

        unsafe {
            gl::ClearColor(0.14f32, 0.14f32, 0.14f32, 1.0f32);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            gl::UseProgram(shader_program.id());
            gl::BindVertexArray(vao_id);

            gl::DrawArrays(gl::TRIANGLES, 0, 3);
        }

        window.gl_swap_window();
    }
}
