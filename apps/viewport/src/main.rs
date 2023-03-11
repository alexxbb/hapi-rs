#![allow(unused)]

use std::default::Default;
use bytemuck::cast_slice;
use eframe::egui::{Context, Key, Modifiers, PointerButton, Sense};
use eframe::epaint::Vertex;
use eframe::{egui, CreationContext, Frame};
use egui::mutex::Mutex;
use egui_glow::CallbackFn;
use glow::HasContext;
use png::Reader;
use std::convert::Into;
use std::ops::{BitXorAssign, Sub};
use std::sync::Arc;
use ultraviolet as uv;
use ultraviolet::{Vec3, Mat4};

#[derive(Copy, Clone)]
struct Camera {
    eye: Vec3,
    look_at: Vec3,
    up_vec: Vec3,
    view: Mat4,
}

impl Camera {
    fn new(eye: Vec3) -> Self {
        let up_vec = Vec3::unit_y();
        let look_at = Vec3::zero();
        let view = Mat4::look_at(eye, look_at, up_vec);
        Camera {
            eye,
            look_at,
            up_vec,
            view
        }
    }
    fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        let rot_y = -delta_x / 150.0;
        let rot_mat = Mat4::from_rotation_y(rot_y);
        self.eye = rot_mat.transform_vec3(self.eye);
        self.view = Mat4::look_at(self.eye, self.look_at, self.up_vec);

        let eye_dir = (self.look_at - self.eye).normalized();
        let ortho = eye_dir.cross(self.up_vec);

        let rot_ortho = -delta_y / 150.0;
        let rot_mat = Mat4::from_rotation_around(ortho.into_homogeneous_vector(), rot_ortho);

        let eye_local = rot_mat.transform_vec3(self.eye - self.look_at);

        let new_eye = eye_local + self.look_at;
        let new_view_dir = self.look_at - new_eye;

        let cos_angle = new_view_dir.dot(self.up_vec) / (new_view_dir.mag() * self.up_vec.mag());

        if cos_angle < 0.95 && cos_angle > -0.95 {
            self.eye = eye_local + self.look_at;
            self.view = Mat4::look_at(self.eye, self.look_at, self.up_vec);
        }

    }

    fn view_matrix(&self) -> Mat4 {
        self.view
    }
}

struct ViewportApp {
    full_screen: bool,
    mesh: Arc<Mutex<Mesh>>,
    scale: f32,
    animated: bool,
    wireframe: bool,
    camera: Camera,
    movement: egui::Vec2,
}

impl ViewportApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let gl = cc.gl.as_ref().expect("Could not init gl Context").clone();
        Self {
            full_screen: false,
            mesh: Arc::new(Mutex::new(Mesh::cube(gl))),
            scale: 1.0,
            animated: true,
            wireframe: false,
            camera: Camera::new(Vec3::new(0.0, 1.0, -2.5)),
            movement: egui::Vec2::splat(0.0)
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
                ui.add(egui::Checkbox::new(&mut self.wireframe, "Wireframe"));
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                ui.allocate_rect(ui.max_rect(), Sense::click());
                let rect = ui.max_rect();
                let aspect_ratio = rect.aspect_ratio();

                let mut mouse_movement = egui::Vec2::splat(0.0);
                ui.input(|input| {
                    if input.pointer.button_down(PointerButton::Primary) {
                        mouse_movement += input.pointer.delta();
                    }
                });

                self.camera.orbit(mouse_movement.x, mouse_movement.y);

                let time = self
                    .animated
                    .then_some(ui.ctx().input(|input| input.time))
                    .unwrap_or(0.0);
                let wireframe = self.wireframe;
                let mesh = Arc::clone(&self.mesh);
                let mut camera = self.camera.clone();
                let callback = egui::PaintCallback {
                    rect: ui.max_rect(),
                    callback: Arc::new(CallbackFn::new(move |info, painter| {
                        mesh.lock().paint(painter.gl(), time, aspect_ratio, &camera, wireframe);
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

unsafe fn compile_gl_program(gl: &glow::Context) -> glow::Program {
    use glow::HasContext as _;

    let program = gl.create_program().expect("gl program");

    let shader_sources = [
        (glow::VERTEX_SHADER, include_str!("cube.vert")),
        (glow::FRAGMENT_SHADER, include_str!("cube.frag")),
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
    gl: Arc<glow::Context>,
    program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    ebo: glow::Buffer,
    texture: glow::Texture,
}

impl Mesh {
    fn cube(gl: Arc<glow::Context>) -> Self {
        Mesh::setup(gl, PLANE, None)
    }

    fn upload(&self, vertices: &[f32]) {
        unsafe {
            self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            self.gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                cast_slice(vertices),
                glow::DYNAMIC_DRAW,
            );
            self.gl.bind_buffer(glow::ARRAY_BUFFER, None);
        }
    }

    fn setup(gl: Arc<glow::Context>, vertices: &[f32], colors: Option<&[f32]>) -> Self {
        use glow::HasContext as _;

        unsafe {
            let program = compile_gl_program(&gl);

            // Create Vertex Array Object. This is the object that describes what and how to
            // draw. Think of it as a preset.
            let vao = gl.create_vertex_array().expect("vertex array");
            gl.bind_vertex_array(Some(vao));

            let vbo = gl.create_buffer().expect("buffer");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, cast_slice(vertices), glow::DYNAMIC_DRAW);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 0, 0);
            gl.enable_vertex_attrib_array(0);



            // UVs
            let uvbo = gl.create_buffer().expect("buffer");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(uvbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, cast_slice(UV), glow::DYNAMIC_DRAW);
            gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, 0, 0);
            gl.enable_vertex_attrib_array(1);

            // Elements
            let ebo = gl.create_buffer().expect("ebo buffer");
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
            gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, cast_slice(INDICES), glow::DYNAMIC_DRAW);


            let decoder = png::Decoder::new(std::fs::File::open("maps/crate.png").unwrap());
            let mut reader = decoder.read_info().unwrap();
            let mut buf = vec![0; reader.output_buffer_size()];
            reader.next_frame(&mut buf).unwrap();
            let (w, h) = (reader.info().width, reader.info().height);

            let texture = gl.create_texture().expect("texture");
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGB as i32,
                w as i32,
                h as i32,
                0,
                glow::RGB,
                glow::UNSIGNED_BYTE,
                Some(&buf),
            );
            gl.generate_mipmap(glow::TEXTURE_2D);

            gl.use_program(Some(program));
            gl.uniform_1_i32(gl.get_uniform_location(program, "myTexture").as_ref(), 0);

            Self {
                gl,
                program,
                vao,
                vbo,
                ebo,
                texture,
            }
        }
    }

    fn paint(&self, gl: &glow::Context, time: f64, aspect_ratio: f32, camera: &Camera, wireframe: bool) {
        use glow::HasContext;

        unsafe {
            gl.enable(glow::DEPTH_TEST);
            gl.clear(glow::DEPTH_BUFFER_BIT);
            gl.bind_vertex_array(Some(self.vao));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            gl.use_program(Some(self.program));

            let push_matrix = |uniform, mat: uv::Mat4| {
                gl.uniform_matrix_4_f32_slice(
                    gl.get_uniform_location(self.program, uniform).as_ref(),
                    false,
                    mat.as_slice(),
                );
            };

            let projection = uv::projection::perspective_gl(45.0, aspect_ratio, 0.01, 10.0);
            push_matrix("projection", projection);

            push_matrix("view", camera.view_matrix());

            let rot = uv::rotor::Rotor3::from_rotation_xz(time as f32 * 0.5);
            let model = uv::Mat4::identity() * rot.into_matrix().into_homogeneous();
            push_matrix("model", model);

            if wireframe {
                gl.polygon_mode(glow::FRONT_AND_BACK, glow::LINE);
            }
            // gl.draw_arrays(glow::TRIANGLES, 0, 36);
            gl.draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_SHORT, 0);
            gl.bind_vertex_array(None);
            gl.polygon_mode(glow::FRONT_AND_BACK, glow::FILL);
        }
    }
}

const PLANE: &[f32] = &[
    -0.5, 0.0, -0.5, // uv0 0 1 0
    0.5, 0.0, -0.5,  // uv1 1 1 0
    -0.5, 0.0, 0.5,  // uv3 0 0 0
    0.5, 0.0, 0.5    // uv2 1 0 0
];

const UV: &[f32] = &[
    0.0, 1.0, 0.0,
    1.0, 1.0, 0.0,
    1.0, 0.0, 0.0,
    0.0, 0.0, 0.0,
];

// Houdini vertex indices
// 0, 1, 3, 2
// Count per face: [4]

const INDICES: &[u16] = &[
    2, 3, 1,

    2, 1, 0
];
// facesFromhoudini = [4, 4, 4, 4,4 ]
//
// triangle_buffer = new Triang;e[num_triangles];
//
// struct Triangle {
//     vec3 points[3];
//     vec2 uvs[3];
//     vec3 normals [3];
// }
//
// int offset = 0;
// for (int face = 0; face < num_faces; face++){
//     num_vertices_in_face = facesFromHoudini[face];
//     num_triangles = num_vertices_in_face - 2;
//     for (int i = 0 ; i < num_triangles; i++) {
//         int triIndexA = vertexIndicesFromHoudini[offset + 0];
//         int triIndexB = vertexIndicesFromHoudini[offset + i + 1];
//         int triIndexC = vertexIndicesFromHoudini[offset + i + 2];
//
//         Triangle tri = {points, uvs, normals};
//         tri.points[0] = PLANE[triIndexA];
//         tri.points[1] = PLANE[triIndexB];
//         tri.points[2] = PLANE[triIndexC];
//
//         tri.uvs[0] = UV[offset + 0];
//         tri.uvs[0] = UV[offset + i + 1];
//         tri.uvs[0] = UV[offset + i + 2];
//     }
//     offset +  + num_vertices_in_face
// }