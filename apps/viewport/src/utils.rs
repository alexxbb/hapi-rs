use bytemuck::cast_slice;
use hapi_rs::attribute::NumericAttr;
use hapi_rs::geometry::AttributeOwner;
use hapi_rs::geometry::Geometry;
use hapi_rs::Result;
use std::mem::size_of;
use std::sync::Arc;

use crate::camera::Camera;

use ultraviolet::{Mat4, Vec2, Vec3};

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Vertex {
    position: Vec3,
    normal: Vec3,
    uv: Vec2,
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

#[derive(Default)]
pub struct MeshData {
    positions: Vec<f32>,
    normals: Option<Vec<f32>>,
    color: Option<Vec<f32>>,
    uvs: Option<Vec<f32>>,
    pub num_vertices: i32,
    pub vertex_array: Vec<Vec3>,
    pub vao: Option<glow::VertexArray>,
    pub vbo: Option<glow::Buffer>,
    pub texture: Option<glow::Texture>,
}

pub struct Renderable {
    pub mesh: MeshData,
    pub program: glow::Program,
}

pub struct Asset {
    pub renderable: Renderable,
    pub node: Option<Geometry>,
    pub gl: Arc<glow::Context>,
}

impl MeshData {
    pub unsafe fn setup_gl(&mut self, gl: Arc<glow::Context>, program: glow::Program) {
        use glow::HasContext as _;

        // Create Vertex Array Object. This is the object that describes what and how to
        // draw. Think of it as a preset.
        let vao = gl.create_vertex_array().expect("vertex array");
        // Generate buffers
        let vbo = gl.create_buffer().expect("buffer");
        // Make VAO current
        gl.bind_vertex_array(Some(vao));

        // Bind VBO
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        // Copy data to it
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            cast_slice(&self.vertex_array),
            glow::STATIC_DRAW,
        );

        // Position
        let mut stride = size_of::<Vec3>();
        if self.normals.is_some() {
            stride += stride;
        }
        if self.uvs.is_some() {
            stride += size_of::<Vec3>();
        }
        gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride as i32, 0);
        // Enable attributes
        gl.enable_vertex_attrib_array(0);

        // Normals
        if self.normals.is_some() {
            let offset = size_of::<Vec3>();
            gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, stride as i32, offset as i32);
            // Enable attributes
            gl.enable_vertex_attrib_array(1);
        }

        // UV
        if self.uvs.is_some() {
            let mut offset = size_of::<Vec3>();
            if self.normals.is_some() {
                offset += size_of::<Vec3>();
            }
            let stride = gl.vertex_attrib_pointer_f32(
                2,
                3,
                glow::FLOAT,
                false,
                stride as i32,
                offset as i32,
            );
            // Enable attributes
            gl.enable_vertex_attrib_array(2);
        }

        let texture = gl.create_texture().expect("texture");
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);

        let (texture_pixels, width, height) = match self.uvs {
            Some(_) => {
                let decoder = png::Decoder::new(std::fs::File::open("maps/crate.png").unwrap());
                let mut reader = decoder.read_info().unwrap();
                let mut pixels = vec![0; reader.output_buffer_size()];
                reader.next_frame(&mut pixels).unwrap();
                let (w, h) = (reader.info().width, reader.info().height);
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
                (pixels, w, h)
            }
            None => {
                // TODO: Not working when there's no uv coords.
                let pixels = vec![
                    0xff, 0x00, 0x00, 0xff, 0x00, 0xff, 0x00, 0xff, 0x00, 0x00, 0xff, 0xff, 0xff,
                    0xff, 0x00, 0xff,
                ];
                let pixels: Vec<u8> = bytemuck::cast_slice(&[1.0, 1.0, 1.0]).into();
                (pixels, 1, 1)
            }
        };

        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGB as i32,
            width as i32,
            height as i32,
            0,
            glow::RGB,
            glow::UNSIGNED_BYTE,
            Some(&texture_pixels),
        );
        gl.generate_mipmap(glow::TEXTURE_2D);

        gl.use_program(Some(program));

        self.vao = Some(vao);
        self.vbo = Some(vbo);
        self.texture = Some(texture);
    }
    pub fn from_houdini_geo(geo: &Geometry) -> Result<Self> {
        let _part = geo.part_info(0)?;
        let _part_id = _part.part_id();
        let positions = geo.get_position_attribute(_part_id)?.get(_part_id)?;
        let face_counts = geo.get_face_counts(Some(&_part))?;
        let vertex_list = geo.vertex_list(Some(&_part))?;
        let uvs = {
            match geo.get_attribute(_part_id, AttributeOwner::Vertex, "uv")? {
                Some(uv_attr) => Some(
                    uv_attr
                        .downcast::<NumericAttr<f32>>()
                        .expect("uv is NumericAttribute")
                        .get(_part_id)?,
                ),
                None => None,
            }
        };
        let normals = {
            match geo.get_attribute(_part_id, AttributeOwner::Vertex, "N")? {
                Some(n_attr) => Some(
                    n_attr
                        .downcast::<NumericAttr<f32>>()
                        .expect("N is NumericAttribute")
                        .get(_part_id)?,
                ),
                None => None,
            }
        };

        let num_vertices = (face_counts.iter().sum::<i32>() / 2) * 3;
        let mut vertex_array = Vec::with_capacity(num_vertices as usize);

        let mut offset = 0;

        for vertex_count_per_face in face_counts {
            let num_triangles = (vertex_count_per_face - 2) as usize;
            for i in 0..num_triangles {
                let off0 = offset + 0;
                let off1 = offset + i + 1;
                let off2 = offset + i + 2;

                let tri_a = vertex_list[off0] as usize;
                let tri_b = vertex_list[off1] as usize;
                let tri_c = vertex_list[off2] as usize;

                let pos_a = Vec3::new(
                    positions[tri_a * 3 + 0],
                    positions[tri_a * 3 + 1],
                    positions[tri_a * 3 + 2],
                );
                let pos_b = Vec3::new(
                    positions[tri_b * 3 + 0],
                    positions[tri_b * 3 + 1],
                    positions[tri_b * 3 + 2],
                );
                let pos_c = Vec3::new(
                    positions[tri_c * 3 + 0],
                    positions[tri_c * 3 + 1],
                    positions[tri_c * 3 + 2],
                );

                // VTX 1
                vertex_array.push(pos_a);
                if let Some(ref normals) = normals {
                    vertex_array.push(Vec3::new(
                        normals[off0 * 3 + 0],
                        normals[off0 * 3 + 1],
                        normals[off0 * 3 + 2],
                    ));
                }
                if let Some(ref uvs) = uvs {
                    vertex_array.push(Vec3::new(uvs[off0 * 3 + 0], uvs[off0 * 3 + 1], 0.0));
                }

                // VTX 2
                vertex_array.push(pos_b);
                if let Some(ref normals) = normals {
                    vertex_array.push(Vec3::new(
                        normals[off1 * 3 + 0],
                        normals[off1 * 3 + 1],
                        normals[off1 * 3 + 2],
                    ));
                }
                if let Some(ref uvs) = uvs {
                    vertex_array.push(Vec3::new(uvs[off1 * 3 + 0], uvs[off1 * 3 + 1], 0.0));
                }

                // VTX 3
                vertex_array.push(pos_c);
                if let Some(ref normals) = normals {
                    vertex_array.push(Vec3::new(
                        normals[off2 * 3 + 0],
                        normals[off2 * 3 + 1],
                        normals[off2 * 3 + 2],
                    ));
                }
                if let Some(ref uvs) = uvs {
                    vertex_array.push(Vec3::new(uvs[off2 * 3 + 0], uvs[off2 * 3 + 1], 0.0));
                }
            }
            offset += vertex_count_per_face as usize;
        }

        // dbg!(&vertex_array);

        dbg!(num_vertices);
        Ok(Self {
            positions,
            normals,
            color: None,
            uvs,
            vertex_array,
            vao: None,
            vbo: None,
            texture: None,
            num_vertices: num_vertices as i32,
        })
    }
}

unsafe fn compile_gl_program(gl: &glow::Context) -> glow::Program {
    use glow::HasContext as _;

    let program = gl.create_program().expect("gl program");

    let shader_sources = [
        (glow::VERTEX_SHADER, include_str!("glsl/shader.vert")),
        (glow::FRAGMENT_SHADER, include_str!("glsl/shader.frag")),
        (glow::GEOMETRY_SHADER, include_str!("glsl/shader.geom")),
    ];
    let shaders: Vec<_> = shader_sources
        .into_iter()
        .map(|(s_type, s_source)| {
            let shader_type = match s_type {
                glow::VERTEX_SHADER => "vertex",
                glow::FRAGMENT_SHADER => "fragment",
                glow::GEOMETRY_SHADER => "geometry",
                _ => unreachable!("Unknown shader type"),
            };
            let shader = gl.create_shader(s_type).expect("Cannot create shader");
            gl.shader_source(shader, s_source);
            gl.compile_shader(shader);
            assert!(
                gl.get_shader_compile_status(shader),
                "Failed to compile \"{shader_type}\" shader: {}",
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

impl Asset {
    pub fn load_hda(gl: Arc<glow::Context>, hda: &str) -> Result<Self> {
        let session = hapi_rs::session::connect_to_pipe("hapi", None, None)?;
        let lib = session.load_asset_file(hda)?;
        let asset = lib.try_create_first()?;
        let geo = asset.geometry()?.expect("Geometry");
        geo.node.cook()?;
        let mut mesh = MeshData::from_houdini_geo(&geo)?;
        let program = unsafe {
            let program = compile_gl_program(&gl);
            mesh.setup_gl(gl.clone(), program);
            program
        };

        Ok(Self {
            gl,
            renderable: Renderable { mesh, program },
            node: Some(geo),
        })
    }

    pub fn paint(&self, camera: &Camera) {
        use glow::HasContext;

        unsafe {
            self.gl.enable(glow::DEPTH_TEST);
            self.gl.clear(glow::DEPTH_BUFFER_BIT);
            self.gl.front_face(glow::CW);
            self.gl.bind_vertex_array(self.renderable.mesh.vao);
            self.gl.active_texture(glow::TEXTURE0);
            self.gl
                .bind_texture(glow::TEXTURE_2D, self.renderable.mesh.texture);
            self.gl.use_program(Some(self.renderable.program));

            let push_matrix = |uniform, mat: Mat4| {
                self.gl.uniform_matrix_4_f32_slice(
                    self.gl
                        .get_uniform_location(self.renderable.program, uniform)
                        .as_ref(),
                    false,
                    mat.as_slice(),
                );
            };

            push_matrix("projection", camera.projection_matrix());
            push_matrix("view", camera.view_matrix());
            push_matrix("model", Mat4::identity());

            self.gl.uniform_3_f32_slice(
                self.gl
                    .get_uniform_location(self.renderable.program, "cameraPos")
                    .as_ref(),
                camera.position().as_slice(),
            );

            self.gl
                .draw_arrays(glow::TRIANGLES, 0, self.renderable.mesh.num_vertices);
            self.gl.bind_vertex_array(None);
        }
    }
}
