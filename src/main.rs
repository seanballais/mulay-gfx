extern crate gl;
extern crate sdl2;

mod assets;
mod c_bridge;
mod graphics;
mod ui;

use std::mem;
use std::os;
use std::ptr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::GLProfile;

use graphics::Program;

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

    let mut app_ui = ui::UI::new(&window);

    // Set up data.
    let vertices = vec![
        -0.25f32, -0.25f32, 0.0f32, 0.25f32, -0.25f32, 0.0f32, 0.0f32, 0.25f32, 0.0f32,
    ];

    let mut shader_asset_manager = match assets::AssetManager::<assets::Shader>::new() {
        Ok(manager) => manager,
        Err(error) => panic!("{:?}", error), // For now. Maybe.
    };
    let vertex_shader =
        match shader_asset_manager.load_asset("vertex-shader", "assets/shaders/triangle.vert") {
            Ok(ptr) => ptr,
            Err(err) => panic!("{:?}", err), // For now. Maybe.
        };
    let fragment_shader =
        match shader_asset_manager.load_asset("fragment-shader", "assets/shaders/triangle.frag") {
            Ok(ptr) => ptr,
            Err(err) => panic!("{:?}", err), // For now. Maybe.
        };

    let mut watcher = match assets::AssetsWatcher::new() {
        Ok(watcher) => watcher,
        Err(error) => panic!("{:?}", error), // For now. Maybe.
    };
    watcher.add_paths_to_watchlist(&vec![
        "assets/shaders/triangle.vert",
        "assets/shaders/triangle.frag",
    ]);

    let shader_program: Arc<Mutex<Program>> = match Program::new(vec![
        Arc::clone(&vertex_shader),
        Arc::clone(&fragment_shader),
    ]) {
        Ok(program) => Arc::new(Mutex::new(program)),
        Err(err) => panic!("{:?}", err),
    };

    let shader_program_ptr1 = Arc::clone(&shader_program);
    let shader_program_ptr2 = Arc::clone(&shader_program);
    shader_asset_manager.register_asset_reload_callback("vertex-shader", move || {
        shader_program_ptr1.lock().unwrap().reload().unwrap();
    });
    shader_asset_manager.register_asset_reload_callback("fragment-shader", move || {
        shader_program_ptr2.lock().unwrap().reload().unwrap();
    });

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

    let mut app_time_start = Instant::now();
    let mut frame_time_start = Instant::now();
    let mut frame_time_end = Instant::now();

    // Event Process
    let mut event_pump = sdl_context.event_pump().unwrap();

    loop {
        let time_since_last_frame = frame_time_end - frame_time_start;
        frame_time_start = Instant::now();

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
                _ => app_ui.process_input(&window, event),
            };
        }

        if do_quit {
            break;
        }

        // Hot-reload.
        let stale_paths = watcher.get_stale_paths();
        let asset_ids = shader_asset_manager.file_paths_to_asset_ids(&stale_paths);
        match shader_asset_manager.reload_assets_by_id(&asset_ids) {
            Ok(_) => {}
            Err(error) => panic!("{:?}", error),
        };

        if !stale_paths.is_empty() {
            watcher.clear_stale_paths();
        }

        unsafe {
            gl::ClearColor(0.14f32, 0.14f32, 0.14f32, 1.0f32);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            gl::UseProgram(shader_program.lock().unwrap().id());
            match shader_program
                .lock()
                .unwrap()
                .add_uniform1f("elapsedTime", app_time_start.elapsed().as_secs_f32())
            {
                Ok(_) => {}
                Err(error) => panic!("{:?}", error),
            };

            gl::BindVertexArray(vao_id);

            gl::DrawArrays(gl::TRIANGLES, 0, 3);
        }

        app_ui.draw_frames(&window, app_time_start.elapsed().as_secs_f64());

        window.gl_swap_window();

        frame_time_end = Instant::now();
    }
}
