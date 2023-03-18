use bytemuck::cast_slice;
use hapi_rs::attribute::NumericAttr;
use hapi_rs::geometry::AttributeOwner;
use hapi_rs::geometry::Geometry;
use hapi_rs::node::Session;
use hapi_rs::session::HoudiniNode;
use hapi_rs::Result;
use std::collections::HashMap;
use std::default::Default;
use std::mem::size_of;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::camera::Camera;
use crate::parameters::{build_parm_map, UiParameter};

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
    colors: Option<Vec<f32>>,
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

pub struct AssetParameters(pub Vec<(String, UiParameter)>);

impl AssetParameters {
    pub fn from_node(node: &HoudiniNode) -> Result<Self> {
        Ok(Self(build_parm_map(node.parameters()?)?))
    }
}

#[derive(Copy, Clone, Default)]
pub struct CookingStats {
    pub cook_count: u64,
    pub avg_cooking_time: Duration,
    pub accum_cooking_time: Duration,
    pub last_cooking_time: Duration,
}

#[derive(Clone, Default)]
pub struct Stats {
    pub cooking: CookingStats,
    pub buffer: BufferStats,
}

impl Stats {
    pub fn reset(&mut self) {
        self.cooking.reset();
        self.buffer.reset();
    }
}

#[derive(Default, Clone)]
pub struct BufferStats {
    pub avg_buffer_time: Duration,
    pub avg_hapi_time: Duration,
    pub mesh_vertex_count: u64,
    accum_buffer_time: Duration,
    accum_hapi_time: Duration,
    last_buffer_time: Duration,
    last_hapi_time: Duration,
}

impl BufferStats {
    pub fn reset(&mut self) {
        *self = Self::default()
    }
    pub fn add_hapi_time(&mut self, time: Duration) {
        self.last_hapi_time = time;
        self.accum_hapi_time += time;
    }

    pub fn last_hapi_time(&mut self, time: Duration) {
        self.last_hapi_time = time;
        self.accum_hapi_time += time;
    }
}

impl CookingStats {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

pub struct Asset {
    pub renderable: Renderable,
    pub asset_node: HoudiniNode,
    pub geometry_node: Geometry,
    pub gl: Arc<glow::Context>,
    pub stats: Stats,
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
            glow::DYNAMIC_DRAW,
        );

        // Position
        let mut stride = size_of::<Vec3>();
        if self.normals.is_some() {
            stride += stride;
        }
        if self.colors.is_some() {
            stride += size_of::<Vec3>();
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

        // Color
        if self.colors.is_some() {
            let mut offset = size_of::<Vec3>();
            if self.normals.is_some() {
                offset += size_of::<Vec3>();
            }
            gl.vertex_attrib_pointer_f32(2, 3, glow::FLOAT, false, stride as i32, offset as i32);
            // Enable attributes
            gl.enable_vertex_attrib_array(2);
        } else {
            gl.disable_vertex_attrib_array(2);
        }

        // UV
        if self.uvs.is_some() {
            let mut offset = size_of::<Vec3>();
            if self.normals.is_some() {
                offset += size_of::<Vec3>();
            }
            let stride = gl.vertex_attrib_pointer_f32(
                3,
                3,
                glow::FLOAT,
                false,
                stride as i32,
                offset as i32,
            );
            // Enable attributes
            gl.enable_vertex_attrib_array(3);
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
    pub fn from_houdini_geo(geo: &Geometry) -> Result<(Self, Duration, Duration)> {
        let _start = Instant::now();
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
        let (normals, point_normal) = {
            let mut point_normal = false;
            let mut vtx_attr = geo.get_attribute(_part_id, AttributeOwner::Vertex, "N")?;
            if vtx_attr.is_none() {
                point_normal = true;
                vtx_attr = geo.get_attribute(_part_id, AttributeOwner::Point, "N")?;
            }
            let normals = match vtx_attr {
                Some(n_attr) => Some(
                    n_attr
                        .downcast::<NumericAttr<f32>>()
                        .expect("N is NumericAttribute")
                        .get(_part_id)?,
                ),
                None => None,
            };
            (normals, point_normal)
        };

        let (colors, point_color) = {
            let mut point_color = false;
            let mut clr_attr = geo.get_attribute(_part_id, AttributeOwner::Vertex, "Cd")?;
            if clr_attr.is_none() {
                point_color = true;
                clr_attr = geo.get_attribute(_part_id, AttributeOwner::Point, "Cd")?;
            }
            let colors = match clr_attr {
                Some(cd_attr) => Some(
                    cd_attr
                        .downcast::<NumericAttr<f32>>()
                        .expect("Cd is NumericAttribute")
                        .get(_part_id)?,
                ),
                None => None,
            };
            (colors, point_color)
        };

        let hapi_time = Instant::now().duration_since(_start);
        let _start = Instant::now();

        let mut num_vertices = (face_counts.iter().sum::<i32>() / 2) * 3;
        num_vertices *= 3; // Position
        if normals.is_some() {
            num_vertices *= 3;
        }
        if colors.is_some() {
            num_vertices *= 3;
        }
        if uvs.is_some() {
            num_vertices *= 3;
        }

        // pig head:
        //  bound checked:         200 us
        //  unsafe unchecked Rust: 180 us

        let mut vertex_array = Vec::with_capacity(num_vertices as usize);

        let mut offset = 0;

        for vertex_count_per_face in face_counts {
            let num_triangles = (vertex_count_per_face - 2) as usize;
            for i in 0..num_triangles {
                let off0 = offset + 0;
                let off1 = offset + i + 1;
                let off2 = offset + i + 2;

                let point_0_index = unsafe { *vertex_list.get_unchecked(off0) as usize };
                let point_1_index = unsafe { *vertex_list.get_unchecked(off1) as usize };
                let point_2_index = unsafe { *vertex_list.get_unchecked(off2) as usize };

                let pos_a = unsafe {
                    Vec3::new(
                        *positions.get_unchecked(point_0_index * 3 + 0),
                        *positions.get_unchecked(point_0_index * 3 + 1),
                        *positions.get_unchecked(point_0_index * 3 + 2),
                    )
                };
                let pos_b = unsafe {
                    Vec3::new(
                        *positions.get_unchecked(point_1_index * 3 + 0),
                        *positions.get_unchecked(point_1_index * 3 + 1),
                        *positions.get_unchecked(point_1_index * 3 + 2),
                    )
                };
                let pos_c = unsafe {
                    Vec3::new(
                        *positions.get_unchecked(point_2_index * 3 + 0),
                        *positions.get_unchecked(point_2_index * 3 + 1),
                        *positions.get_unchecked(point_2_index * 3 + 2),
                    )
                };

                // VTX 1
                vertex_array.push(pos_a);
                // Normals
                if let Some(ref normals) = normals {
                    let idx = if point_normal { point_0_index } else { off0 };
                    vertex_array.push(unsafe {
                        Vec3::new(
                            *normals.get_unchecked(idx * 3 + 0),
                            *normals.get_unchecked(idx * 3 + 1),
                            *normals.get_unchecked(idx * 3 + 2),
                        )
                    });
                }
                // Color
                if let Some(ref colors) = colors {
                    let idx = if point_color { point_0_index } else { off0 };
                    vertex_array.push(unsafe {
                        Vec3::new(
                            *colors.get_unchecked(idx * 3 + 0),
                            *colors.get_unchecked(idx * 3 + 1),
                            *colors.get_unchecked(idx * 3 + 2),
                        )
                    });
                }

                // UV
                if let Some(ref uvs) = uvs {
                    vertex_array.push(Vec3::new(uvs[off0 * 3 + 0], uvs[off0 * 3 + 1], 0.0));
                }

                // VTX 2
                vertex_array.push(pos_b);
                // Normal
                if let Some(ref normals) = normals {
                    let idx = if point_normal { point_1_index } else { off1 };
                    vertex_array.push(unsafe {
                        Vec3::new(
                            *normals.get_unchecked(idx * 3 + 0),
                            *normals.get_unchecked(idx * 3 + 1),
                            *normals.get_unchecked(idx * 3 + 2),
                        )
                    });
                }

                // Color
                if let Some(ref colors) = colors {
                    let idx = if point_color { point_1_index } else { off1 };
                    vertex_array.push(unsafe {
                        Vec3::new(
                            *colors.get_unchecked(idx * 3 + 0),
                            *colors.get_unchecked(idx * 3 + 1),
                            *colors.get_unchecked(idx * 3 + 2),
                        )
                    });
                }

                // UV
                if let Some(ref uvs) = uvs {
                    vertex_array.push(Vec3::new(uvs[off1 * 3 + 0], uvs[off1 * 3 + 1], 0.0));
                }

                // VTX 3
                vertex_array.push(pos_c);
                // Normal
                if let Some(ref normals) = normals {
                    let idx = if point_normal { point_2_index } else { off2 };
                    vertex_array.push(unsafe {
                        Vec3::new(
                            *normals.get_unchecked(idx * 3 + 0),
                            *normals.get_unchecked(idx * 3 + 1),
                            *normals.get_unchecked(idx * 3 + 2),
                        )
                    });
                }
                // Color
                if let Some(ref colors) = colors {
                    let idx = if point_color { point_2_index } else { off2 };
                    vertex_array.push(unsafe {
                        Vec3::new(
                            *colors.get_unchecked(idx * 3 + 0),
                            *colors.get_unchecked(idx * 3 + 1),
                            *colors.get_unchecked(idx * 3 + 2),
                        )
                    });
                }
                // UV
                if let Some(ref uvs) = uvs {
                    vertex_array.push(Vec3::new(uvs[off2 * 3 + 0], uvs[off2 * 3 + 1], 0.0));
                }
            }
            offset += vertex_count_per_face as usize;
        }

        // eprintln!("Mesh number vertices: {num_vertices}");
        let vertex_processing_time = Instant::now().duration_since(_start);
        let _self = Self {
            positions,
            normals,
            colors,
            uvs,
            vertex_array,
            vao: None,
            vbo: None,
            texture: None,
            num_vertices: num_vertices as i32,
        };
        Ok((_self, hapi_time, vertex_processing_time))
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
    pub fn load_hda(gl: Arc<glow::Context>, session: &Session, hda: &str) -> Result<Self> {
        let lib = session.load_asset_file(hda)?;
        let asset = lib.try_create_first()?;
        let geo = asset.geometry()?.expect("Geometry");
        let _start = Instant::now();
        geo.node.cook()?;
        let cooking_time = Instant::now().duration_since(_start);
        let (mut mesh, hapi_time, vertex_processing_time) = MeshData::from_houdini_geo(&geo)?;
        let buffer_build_time = Instant::now().duration_since(_start);
        let program = unsafe {
            let program = compile_gl_program(&gl);
            mesh.setup_gl(gl.clone(), program);
            program
        };

        let mesh_vertex_count = mesh.positions.len() as u64;
        Ok(Self {
            gl,
            asset_node: asset,
            renderable: Renderable { mesh, program },
            geometry_node: geo,
            stats: Stats {
                cooking: CookingStats {
                    cook_count: 1,
                    avg_cooking_time: Duration::from_micros(0),
                    last_cooking_time: cooking_time,
                    accum_cooking_time: Duration::from_micros(0),
                },
                buffer: BufferStats {
                    mesh_vertex_count,
                    ..Default::default()
                },
            },
        })
    }

    pub fn cook(&mut self) -> Result<()> {
        let _start = Instant::now();
        self.geometry_node.node.cook()?;
        let CookingStats {
            cook_count,
            avg_cooking_time,
            accum_cooking_time,
            last_cooking_time,
        } = &mut self.stats.cooking;
        *cook_count += 1;
        *last_cooking_time = Instant::now().duration_since(_start);
        if *cook_count > 5 {
            *accum_cooking_time += *last_cooking_time;
            *avg_cooking_time = *accum_cooking_time / (*cook_count as u32 - 5);
        } else {
            *avg_cooking_time = Duration::from_micros(0);
        }
        Ok(())
    }

    pub fn rebuild_mesh(&mut self) -> Result<()> {
        let (mesh, hapi_time, processing_time) = MeshData::from_houdini_geo(&self.geometry_node)?;
        let BufferStats {
            avg_buffer_time,
            avg_hapi_time,
            mesh_vertex_count,
            accum_buffer_time,
            accum_hapi_time,
            last_buffer_time,
            last_hapi_time,
        } = &mut self.stats.buffer;
        let cook_count = self.stats.cooking.cook_count;
        *mesh_vertex_count = mesh.positions.len() as u64;
        *last_buffer_time = processing_time;
        *last_hapi_time = hapi_time;
        if cook_count > 5 {
            *accum_buffer_time += *last_buffer_time;
            *accum_hapi_time += *last_hapi_time;
            *avg_buffer_time = *accum_buffer_time / (cook_count as u32 - 5);
            *avg_hapi_time = *accum_hapi_time / (cook_count as u32 - 5);
        } else {
            *avg_buffer_time = Duration::from_micros(0);
            *avg_hapi_time = Duration::from_micros(0);
        }
        self.renderable.mesh = mesh;
        unsafe {
            self.renderable
                .mesh
                .setup_gl(self.gl.clone(), self.renderable.program);
        }
        Ok(())
    }

    pub fn draw(&self, camera: &Camera) {
        use glow::HasContext;

        unsafe {
            self.gl.enable(glow::DEPTH_TEST);
            self.gl.enable(glow::MULTISAMPLE);
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

            let use_color_loc = self
                .gl
                .get_uniform_location(self.renderable.program, "use_point_color");
            self.gl.uniform_1_i32(
                use_color_loc.as_ref(),
                self.renderable.mesh.colors.is_some() as i32,
            );

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
