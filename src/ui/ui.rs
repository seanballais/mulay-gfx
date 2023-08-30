use egui;
use egui_sdl2_gl;
use sdl2;

pub struct UI {
    egui_ctx: egui::Context,
    egui_painter: egui_sdl2_gl::painter::Painter,
    egui_state: egui_sdl2_gl::EguiStateHandler,
    console_contents: String,
    console_command_contents: String,
}

impl UI {
    pub fn new(window: &sdl2::video::Window) -> UI {
        let (egui_painter, egui_state) = egui_sdl2_gl::with_sdl2(
            window,
            egui_sdl2_gl::ShaderVersion::Default,
            egui_sdl2_gl::DpiScaling::Default,
        );
        let egui_ctx = egui::Context::default();

        UI {
            egui_ctx,
            egui_painter,
            egui_state,
            console_contents: String::from(""),
            console_command_contents: String::from(""),
        }
    }

    pub fn draw_frames(&mut self, window: &sdl2::video::Window, app_elapsed_time: f64) {
        self.egui_state.input.time = Some(app_elapsed_time);
        self.egui_ctx.begin_frame(self.egui_state.input.take());

        egui::Window::new("Console").show(&self.egui_ctx, |ui| {
            egui::ScrollArea::vertical()
                .max_width(f32::INFINITY)
                .max_height(256f32)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.add(egui::TextEdit::multiline(
                        &mut self.console_contents.as_str(),
                    ));
                });

            let textedit_response = ui.add(egui::TextEdit::singleline(
                &mut self.console_command_contents,
            ));
            if textedit_response.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                && !self.console_command_contents.is_empty()
            {
                if !self.console_contents.is_empty() {
                    self.console_contents.push_str("\n");
                }
                
                self.console_contents
                    .push_str(&self.console_command_contents.as_str());
                self.console_command_contents.clear();

                textedit_response.request_focus();
            }
        });

        let egui::FullOutput {
            platform_output,
            textures_delta,
            shapes,
            ..
        } = self.egui_ctx.end_frame();

        self.egui_state.process_output(window, &platform_output);
        let paint_jobs = self.egui_ctx.tessellate(shapes);
        self.egui_painter
            .paint_jobs(None, textures_delta, paint_jobs);
    }

    pub fn process_input(&mut self, window: &sdl2::video::Window, event: sdl2::event::Event) {
        self.egui_state
            .process_input(window, event, &mut self.egui_painter);
    }
}
