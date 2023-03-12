#![allow(unused)]

mod utils;
mod camera;

use camera::Camera;
use eframe::egui::{Context, Key, Modifiers, PointerButton, Sense};
use eframe::epaint::Vertex;
use eframe::{egui, CreationContext, Frame};
use egui::mutex::Mutex;
use egui_glow::CallbackFn;
use glow::HasContext;
use png::Reader;
use std::default::Default;
use std::ops::{BitXorAssign, Sub};
use std::sync::Arc;
use ultraviolet as uv;
use ultraviolet::{Mat4, Vec3};
use utils::{Asset, MeshData};

static OTL: &str = r#"C:\Github\hapi-rs\apps\viewport\otls\hapi_cube.hda"#;

struct ViewportApp {
    full_screen: bool,
    asset: Arc<Mutex<Asset>>,
    scale: f32,
    animated: bool,
    camera: Camera,
    movement: egui::Vec2,
}

impl ViewportApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let gl = cc.gl.as_ref().expect("Could not init gl Context").clone();
        let asset = Asset::load_hda(gl, OTL).expect("Load HDA");
        Self {
            full_screen: false,
            asset: Arc::new(Mutex::new(asset)),
            scale: 1.0,
            animated: true,
            camera: Camera::new(Vec3::new(0.0, 1.0, -2.5)),
            movement: egui::Vec2::splat(0.0),
        }
    }
}

impl eframe::App for ViewportApp {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        egui::SidePanel::left("parameters")
            .resizable(true)
            .default_width(400.0)
            .min_width(200.0)
            .show(ctx, |ui| {
                ui.heading("PARAMETERS");
                ui.add(egui::Checkbox::new(&mut self.animated, "Animate"));
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                ui.allocate_rect(ui.max_rect(), Sense::click());
                let rect = ui.max_rect();

                let mut mouse_movement = egui::Vec2::splat(0.0);
                let mut wheel_zoom = 0.0f32;
                ui.input(|input| {
                    if input.pointer.button_down(PointerButton::Primary) {
                        mouse_movement += input.pointer.delta();
                    }
                    if input.pointer.button_down(PointerButton::Secondary) {
                        let delta = input.pointer.delta() * 0.005;
                        wheel_zoom += delta.x + delta.y;
                    }
                });

                self.camera.orbit(mouse_movement.x, mouse_movement.y);
                self.camera.set_aspect_ratio(rect.aspect_ratio());
                self.camera.set_zoom(wheel_zoom);

                let time = self
                    .animated
                    .then_some(ui.ctx().input(|input| input.time))
                    .unwrap_or(0.0);
                let asset = Arc::clone(&self.asset);
                let mut camera = self.camera.clone();
                let callback = egui::PaintCallback {
                    rect: ui.max_rect(),
                    callback: Arc::new(CallbackFn::new(move |info, painter| {
                        let asset = asset.lock();
                        asset.paint(&camera);
                    })),
                };
                ui.painter().add(callback);
                if self.animated {
                    ui.ctx().request_repaint();
                }
            })
        });

        ctx.input(|input| {
            if input.key_pressed(Key::Escape) {
                frame.close()
            }
            if input.modifiers.matches(Modifiers::CTRL) && input.key_pressed(Key::F) {
                self.full_screen.bitxor_assign(true);
            }
            frame.set_fullscreen(self.full_screen);
        });
    }
}

fn main() {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(700.0, 500.0)),
        initial_window_pos: Some(egui::Pos2::new(1000.0, 500.0)),
        multisampling: 8,
        renderer: eframe::Renderer::Glow,
        depth_buffer: 24,
        ..Default::default()
    };
    let creator: eframe::AppCreator = Box::new(move |cc| Box::new(ViewportApp::new(cc)));
    eframe::run_native("HAPI Viewport", options, creator);
}

// fn upload(&self, vertices: &[f32]) {
//     unsafe {
//         self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
//         self.gl.buffer_data_u8_slice(
//             glow::ARRAY_BUFFER,
//             cast_slice(vertices),
//             glow::DYNAMIC_DRAW,
//         );
//         self.gl.bind_buffer(glow::ARRAY_BUFFER, None);
//     }
// }
