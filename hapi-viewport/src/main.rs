use glium::glutin;
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::Arc;

fn create_display(event_loop: &glutin::event_loop::EventLoop<()>) -> glium::Display {
    let window_builder = glutin::window::WindowBuilder::new()
        .with_resizable(true)
        .with_inner_size(glutin::dpi::LogicalSize {
            width: 800.0,
            height: 600.0,
        })
        .with_title("egui_glium example");

    let context_builder = glutin::ContextBuilder::new()
        .with_depth_buffer(0)
        .with_srgb(true)
        .with_stencil_buffer(0)
        .with_vsync(true);

    glium::Display::new(window_builder, context_builder, &event_loop).unwrap()
}

#[derive(Debug)]
struct State {
    color: [f32; 3],
    label: String,
    slider: f32,
}

impl Default for State {
    fn default() -> Self {
        Self {color: [0.2, 0.2, 0.2], label: "".to_string(), slider: 0.0}
    }
}

fn main() {
    let event_loop = glutin::event_loop::EventLoop::with_user_event();
    let display = create_display(&&event_loop);

    let mut egui = egui_glium::EguiGlium::new(&display);
    let mut state = State::default();

    event_loop.run(move |event, _, control_flow| {
        let mut redraw = || {
            let mut quit = false;

            egui.begin_frame(&display);

            egui::TopPanel::top("top_panel").show(egui.ctx(), |ui| {
                // The top panel is often a good place for a menu bar:
                egui::menu::bar(ui, |ui| {
                    egui::menu::menu(ui, "File", |ui| {
                        if ui.button("Quit").clicked() {
                            quit = true;
                        }
                    });
                });
            });

            egui::SidePanel::left("side_panel", 200.0).show(egui.ctx(), |ui| {
                ui.heading("Side Panel");

                ui.horizontal(|ui| {
                    ui.label("Write something: ");
                    ui.text_edit_singleline(&mut state.label);
                });

                ui.add(egui::Slider::new(&mut state.slider, 0.0..=10.0).text("value"));
                ui.color_edit_button_rgb(&mut state.color);
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add(
                        egui::Hyperlink::new("https://github.com/emilk/egui/").text("powered by egui"),
                    );
                });
            });

            let (needs_repaint, shapes) = egui.end_frame(&display);

            *control_flow = if quit {
                glutin::event_loop::ControlFlow::Exit
            } else if needs_repaint {
                display.gl_window().window().request_redraw();
                glutin::event_loop::ControlFlow::Poll
            } else {
                glutin::event_loop::ControlFlow::Wait
            };

            {
                use glium::Surface as _;
                let mut target = display.draw();

                // draw things behind egui here

                let clear_color = egui::Rgba::from_rgb(0.1, 0.3, 0.2);
                let [r,g,b] = state.color;

                target.clear_color(
                    r,
                    g,
                    b,
                    1.0,
                );
                egui.paint(&display, &mut target, shapes);


                // draw things on top of egui here

                target.finish().unwrap();
            }
        };

        match event {
            // Platform-dependent event handlers to workaround a winit bug
            // See: https://github.com/rust-windowing/winit/issues/987
            // See: https://github.com/rust-windowing/winit/issues/1619
            glutin::event::Event::RedrawEventsCleared if cfg!(windows) => redraw(),
            glutin::event::Event::RedrawRequested(_) if !cfg!(windows) => redraw(),

            glutin::event::Event::WindowEvent { event, .. } => {
                egui.on_event(event, control_flow);
                display.gl_window().window().request_redraw(); // TODO: ask egui if the events warrants a repaint instead
            }

            _ => (),
        }
    });
}