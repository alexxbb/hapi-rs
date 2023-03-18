#![allow(unused)]

mod ogl;
mod camera;
mod parameters;

use camera::Camera;
use eframe::egui::{Color32, Context, Key, Modifiers, PointerButton, Sense};
use eframe::epaint::Vertex;
use eframe::{egui, CreationContext, Frame};
use egui::mutex::Mutex;
use egui_glow::CallbackFn;
use glow::HasContext;
use path_absolutize::Absolutize;
use std::default::Default;
use std::ops::{BitXorAssign, Sub};
use std::sync::Arc;
use std::time::Duration;
use ultraviolet as uv;
use ultraviolet::{Mat4, Vec3};

use hapi_rs::parameter::Parameter;

use crate::parameters::{ParmKind, UiParameter};
use ogl::{Asset, AssetParameters, BufferStats, CookingStats, MeshData, Stats};

static OTL: &str = "otls/hapi_opengl.hda";
static ICON: &str = "maps/icon.png";

struct ViewportApp {
    full_screen: bool,
    asset: Arc<Mutex<Asset>>,
    asset_parameters: Arc<Mutex<AssetParameters>>,
    scale: f32,
    turntable: bool,
    camera: Camera,
    movement: egui::Vec2,
}

impl ViewportApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let gl = cc.gl.as_ref().expect("Could not init gl Context").clone();
        let session = if cfg!(debug_assertions) {
            if let Ok(session) = hapi_rs::session::connect_to_pipe("hapi", None, None) {
                session
            } else {
                eprintln!("Could not connect to HARS server, starting new in-process");
                hapi_rs::session::new_in_process(None).expect("in-process session")
            }
        } else {
            eprintln!("Starting in-process session");
            hapi_rs::session::new_in_process(None).expect("Houdini session")
        };
        let otl = std::path::Path::new(OTL).absolutize().expect("OTL path");
        let otl = otl.to_string_lossy();
        let asset = Asset::load_hda(gl, &session, &otl).expect("Load HDA");
        let asset_parameters =
            AssetParameters::from_node(&asset.asset_node).expect("Asset parameters");

        Self {
            full_screen: false,
            asset: Arc::new(Mutex::new(asset)),
            asset_parameters: Arc::new(Mutex::new(asset_parameters)),
            scale: 1.0,
            turntable: false,
            camera: Camera::new(Vec3::new(0.0, 1.0, -2.0)),
            movement: egui::Vec2::splat(0.0),
        }
    }
}

impl eframe::App for ViewportApp {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        egui::SidePanel::left("parameters")
            .resizable(true)
            .default_width(200.0)
            .min_width(200.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Asset Parameters");
                });
                ui.separator();
                // ui.add(egui::Checkbox::new(&mut self.turntable, "Turntable"));
                let mut asset = self.asset.lock();

                let mut rebuild_fn = |reset_stats| {
                    asset.cook().expect("Cook");
                    asset.rebuild_mesh().expect("rebuild");
                    if reset_stats {
                        asset.stats.reset();
                    }
                    ctx.request_repaint();
                };

                let mut parameters = &mut self.asset_parameters.lock();
                for (parm_name, ui_parm) in parameters.0.iter_mut() {
                    let UiParameter {
                        parameter: hou_parm,
                        kind: parm_kind,
                    } = ui_parm;
                    match parm_kind {
                        ParmKind::Menu { choices, current } => {
                            let current_selection = &choices[*current as usize];
                            egui::ComboBox::from_label(parm_name.as_str())
                                .selected_text(current_selection)
                                .show_ui(ui, |ui| {
                                    for (i, choice) in choices.iter().enumerate() {
                                        if ui.selectable_value(current, i as i32, choice).changed()
                                        {
                                            if let Parameter::Int(p) = &hou_parm {
                                                p.set(0, *current).expect("Parameter Update");
                                                rebuild_fn(true);
                                            }
                                        }
                                    }
                                });
                        }
                        ParmKind::Float { ref mut current } => {
                            if ui.add(egui::Slider::new(current, 0.0..=10.0)).changed() {
                                if let Parameter::Float(p) = &hou_parm {
                                    p.set(0, *current).expect("Parameter Update");
                                    rebuild_fn(false);
                                }
                            }
                        }
                        ParmKind::Toggle { ref mut current } => {
                            if ui.checkbox(current, parm_name.as_str()).changed() {
                                if let Parameter::Int(p) = &hou_parm {
                                    p.set(0, *current as i32).expect("Parameter Update");
                                    rebuild_fn(false);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                ui.separator();
                ui.vertical_centered(|ui| {
                    ui.label("Stats");
                });
                ui.group(|ui| {
                    let Stats { cooking, buffer } = &asset.stats;
                    use egui::ecolor::Color32;
                    use egui::epaint::text::TextFormat;
                    use egui::epaint::Stroke;
                    use egui::text::LayoutJob;

                    macro_rules! stat {
                        ($text:literal, $value:expr) => {
                            let mut job = LayoutJob::default();
                            job.append(
                                $text,
                                0.0,
                                TextFormat {
                                    color: Color32::GRAY,
                                    italics: true,
                                    ..Default::default()
                                },
                            );
                            job.append(
                                $value,
                                0.0,
                                TextFormat {
                                    color: Color32::WHITE,
                                    ..Default::default()
                                },
                            );

                            ui.label(job);
                        };
                    }

                    let CookingStats {
                        cook_count,
                        avg_cooking_time,
                        ..
                    } = &cooking;
                    let BufferStats {
                        avg_buffer_time,
                        avg_hapi_time,
                        mesh_vertex_count,
                        ..
                    } = &buffer;
                    stat!("Vertex Count:", &format!("           {mesh_vertex_count}"));
                    ui.separator();
                    stat!("Cook Count:", &format!("           {cook_count}"));
                    stat!(
                        "Avg Cooking Time:",
                        &format!("      {} ms", avg_cooking_time.as_millis())
                    );
                    stat!(
                        "Avg Mesh Time:",
                        &format!("            {} μs", avg_buffer_time.as_micros())
                    );
                    stat!(
                        "Avg API Time:",
                        &format!("                {} μs", avg_hapi_time.as_micros())
                    );
                });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                ui.allocate_rect(ui.max_rect(), Sense::click());
                let rect = ui.max_rect();

                let mut mouse_movement = egui::Vec2::splat(0.0);
                let mut wheel_zoom = 0.0f32;
                ui.input(|input| {
                    if input.pointer.button_down(PointerButton::Primary) {
                        if rect.contains(input.pointer.hover_pos().expect("Cursor position")) {
                            mouse_movement += input.pointer.delta();
                        }
                    }
                    if input.pointer.button_down(PointerButton::Secondary) {
                        if rect.contains(input.pointer.hover_pos().expect("Cursor position")) {
                            let delta = input.pointer.delta() * 0.005;
                            wheel_zoom += delta.x + delta.y;
                        }
                    }
                });

                self.camera.set_aspect_ratio(rect.aspect_ratio());
                self.camera.set_zoom(wheel_zoom);
                if self.turntable {
                    // TODO rotate camera
                    let time = self
                        .turntable
                        .then_some(ui.ctx().input(|input| input.time))
                        .unwrap_or(0.0);
                } else {
                    self.camera.orbit(mouse_movement.x, mouse_movement.y);
                }

                let asset = Arc::clone(&self.asset);
                let mut camera = self.camera.clone();
                let callback = egui::PaintCallback {
                    rect: ui.max_rect(),
                    callback: Arc::new(CallbackFn::new(move |info, painter| {
                        let asset = asset.lock();
                        asset.draw(&camera);
                    })),
                };
                ui.painter().add(callback);
                if self.turntable {
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

fn load_icon() -> Option<eframe::IconData> {
    let Ok(file) = std::fs::File::open(ICON) else {
        eprintln!("Could not load app icon");
        return None
    };
    let decoder = png::Decoder::new(file);
    let mut reader = decoder.read_info().expect("png reader");
    let mut pixels = vec![0; reader.output_buffer_size()];
    let width = reader.info().width;
    let height = reader.info().height;
    reader.next_frame(&mut pixels).unwrap();

    Some(eframe::IconData {
        rgba: pixels,
        width,
        height,
    })
}

fn main() {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1200.0, 800.0)),
        initial_window_pos: Some(egui::Pos2::new(1000.0, 500.0)),
        multisampling: 16,
        renderer: eframe::Renderer::Glow,
        depth_buffer: 24,
        icon_data: load_icon(),
        ..Default::default()
    };
    let creator: eframe::AppCreator = Box::new(move |cc| Box::new(ViewportApp::new(cc)));
    eframe::run_native("HAPI Viewport", options, creator);
}
