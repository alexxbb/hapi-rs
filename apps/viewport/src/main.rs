#![allow(unused)]

use bytemuck::cast_slice;
use eframe::egui::{Context, Key, Modifiers, Sense};
use eframe::epaint::Vertex;
use eframe::{egui, CreationContext, Frame};
use egui::mutex::Mutex;
use std::ops::BitXorAssign;
use std::sync::Arc;
use ultraviolet as uv;

#[derive(Debug)]
struct Cube {
    program: glow::Program,
    vertex_array: glow::VertexArray,
}

impl Cube {
    fn new(gl: &glow::Context) -> Self {
        use glow::HasContext as _;

        unsafe {
            let program = gl.create_program().expect("gl program");

            let (vtx_source, frag_source) = (
                r#"
                    #version 330
                    
                    layout (location = 0) in vec3 pos; 
                    void main() {
                        gl_Position = vec4(pos.x, pos.y, pos.z, 1.0);
                    }
                "#,
                r#"
                    #version 330
                    
                    out vec4 out_color;
                    
                    void main() {
                        out_color = vec4(1.0f, 0.5f, 0.2f, 1.0f);
                    } 
                "#,
            );
            let shader_sources = [
                (glow::VERTEX_SHADER, vtx_source),
                (glow::FRAGMENT_SHADER, frag_source),
            ];
            let shaders: Vec<_> = shader_sources
                .into_iter()
                .map(|(s_type, s_source)| {
                    let shader = gl.create_shader(s_type).expect("Cannot create shader");
                    gl.shader_source(shader, s_source);
                    gl.compile_shader(shader);
                    assert!(
                        gl.get_shader_compile_status(shader),
                        "Failed to compile shader {s_type}: {}",
                        gl.get_shader_info_log(shader)
                    );
                    gl.attach_shader(program, shader);
                    shader
                })
                .collect();
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!(
                    "GL Program link error: {}",
                    gl.get_program_info_log(program)
                );
            }
            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader)
            }

            let vertices: [uv::Vec3; 3] = [
                [-0.5, -0.5, 0.0].into(),
                [0.5, -0.5, 0.0].into(),
                [0.0, 0.5, 0.0].into(),
            ];

            // Generate new buffer object
            let vbo = gl.create_buffer().expect("buffer");

            // Create Vertex Array Object
            let vertex_array = gl.create_vertex_array().expect("vertex array");
            // Make it current
            gl.bind_vertex_array(Some(vertex_array));

            // Bind buffer to it
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));

            // Copy data to the buffer
            let data = cast_slice(&vertices);
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, data, glow::STATIC_DRAW);

            // Create attribute pointer at location(0) in shader
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 0, 0);

            // Enable this attribute
            gl.enable_vertex_attrib_array(0);

            Self {
                program,
                vertex_array,
            }
        }
    }

    fn destroy(&self, gl: &glow::Context) {
        use glow::HasContext;

        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vertex_array);
        }
    }

    fn paint(&self, gl: &glow::Context) {
        use glow::HasContext;

        unsafe {
            gl.polygon_mode(glow::FRONT_AND_BACK, glow::LINE);
            gl.use_program(Some(self.program));
            gl.bind_vertex_array(Some(self.vertex_array));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
            gl.polygon_mode(glow::FRONT_AND_BACK, glow::FILL);
        }
    }
}

struct ViewportApp {
    full_screen: bool,
    mesh: Arc<Mutex<Cube>>,
}

impl ViewportApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let gl = cc.gl.as_ref().expect("Could not init gl Context");
        Self {
            full_screen: false,
            mesh: Arc::new(Mutex::new(Cube::new(gl))),
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
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                let resp = ui.allocate_rect(ui.max_rect(), Sense::click());

                let mesh = self.mesh.clone();
                let callback = egui::PaintCallback {
                    rect: ui.max_rect(),
                    callback: Arc::new(egui_glow::CallbackFn::new(move |info, painter| {
                        mesh.lock().paint(painter.gl());
                    })),
                };
                ui.painter().add(callback);
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
        multisampling: 4,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    let creator: eframe::AppCreator = Box::new(move |cc| Box::new(ViewportApp::new(cc)));
    eframe::run_native("HAPI Viewport", options, creator);
}

pub unsafe fn to_byte_slice<T>(slice: &[T]) -> &[u8] {
    std::slice::from_raw_parts(
        slice.as_ptr().cast(),
        slice.len() * std::mem::size_of::<T>(),
    )
}
