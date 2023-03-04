#![allow(unused)]

use bytemuck::cast_slice;
use eframe::egui::{Context, Key, Modifiers, Sense};
use eframe::epaint::Vertex;
use eframe::{egui, CreationContext, Frame};
use egui::mutex::Mutex;
use std::ops::BitXorAssign;
use std::sync::Arc;
use ultraviolet as uv;

struct ViewportApp {
    full_screen: bool,
    meshes: Arc<Mutex<Vec<Shapes>>>,
    draw_index: usize,
}

impl ViewportApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let gl = cc.gl.as_ref().expect("Could not init gl Context");
        Self {
            full_screen: false,
            meshes: Arc::new(Mutex::new(vec![
                Shapes::Square(Mesh::square(gl)),
                Shapes::Triangle(Mesh::triangle(gl)),
            ])),
            draw_index: 0,
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
                let label = if self.draw_index == 0 {
                    "Square"
                } else {
                    "Tri"
                };
                egui::ComboBox::from_label("")
                    .selected_text(label)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.draw_index, 0, "Square");
                        ui.selectable_value(&mut self.draw_index, 1, "Tri");
                    });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                let resp = ui.allocate_rect(ui.max_rect(), Sense::click());

                let meshes = Arc::clone(&self.meshes);
                let draw_index = self.draw_index;
                let callback = egui::PaintCallback {
                    rect: ui.max_rect(),
                    callback: Arc::new(egui_glow::CallbackFn::new(move |info, painter| {
                        let mesh = &meshes.lock()[draw_index];
                        match mesh {
                            Shapes::Triangle(m) => m.paint(painter.gl()),
                            Shapes::Square(m) => m.paint(painter.gl()),
                        }
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

unsafe fn compile_gl_program(gl: &glow::Context) -> glow::Program {
    use glow::HasContext as _;

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
    program
}

struct Mesh {
    program: glow::Program,
    vao: glow::VertexArray,
    num_vtx: i32,
}

impl Mesh {
    fn triangle(gl: &glow::Context) -> Self {
        #[rustfmt::skip]
        let vertices = &[
            -0.5f32, -0.5, 0.0,
            0.5, -0.5, 0.0,
            0.5, 0.5, 0.0
        ];
        let indices = &[0, 1, 2];

        Mesh::new(gl, vertices.as_slice(), indices.as_slice())
    }

    fn square(gl: &glow::Context) -> Self {
        #[rustfmt::skip]
        let vertices = &[
            -0.5, -0.5, 0.0,
            0.5, -0.5, 0.0,
            0.5, 0.5, 0.0,
            -0.5, 0.5, 0.0,
        ];
        let indices = &[0, 1, 2, 2, 3, 0];

        Mesh::new(gl, vertices.as_slice(), indices.as_slice())
    }

    fn new(gl: &glow::Context, vertices: &[f32], indices: &[i32]) -> Self {
        use glow::HasContext as _;

        unsafe {
            let program = compile_gl_program(gl);

            // Create Vertex Array Object. This is the object that describes what and how to
            // draw. Think of it as a preset.
            let vao = gl.create_vertex_array().expect("vertex array");
            // Make it current
            gl.bind_vertex_array(Some(vao));

            // Generate buffers
            let vbo = gl.create_buffer().expect("buffer");
            let ebo = gl.create_buffer().expect("ebo buffer");

            // Bind VBO
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            // Copy data to it
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, cast_slice(vertices), glow::STATIC_DRAW);

            // Bind EBO
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
            // Copy data to it
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                cast_slice(indices),
                glow::STATIC_DRAW,
            );

            // Create attribute pointer at location(0) in shader
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 0, 0);

            // Enable this attribute
            gl.enable_vertex_attrib_array(0);

            Self {
                program,
                vao,
                num_vtx: vertices.len() as i32,
            }
        }
    }

    fn paint(&self, gl: &glow::Context) {
        use glow::HasContext;

        unsafe {
            gl.polygon_mode(glow::FRONT_AND_BACK, glow::LINE);
            gl.use_program(Some(self.program));
            gl.bind_vertex_array(Some(self.vao));
            gl.draw_elements(glow::TRIANGLES, self.num_vtx, glow::UNSIGNED_INT, 0);
            gl.polygon_mode(glow::FRONT_AND_BACK, glow::FILL);
            gl.bind_vertex_array(None);
        }
    }
}

enum Shapes {
    Triangle(Mesh),
    Square(Mesh),
}
