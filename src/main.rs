extern crate gl;
extern crate sdl2;

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

    let ctx = window.gl_create_context().unwrap();
    gl::load_with(|name| video_subsystem.gl_get_proc_address(name) as *const _);

    debug_assert_eq!(gl_attr.context_profile(), GLProfile::Core);
    debug_assert_eq!(gl_attr.context_version(), (3, 3));

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
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        window.gl_swap_window();
    }
}
