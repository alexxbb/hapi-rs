use hapi_rs::attribute::NumericAttr;
use hapi_rs::geometry::AttributeOwner;
use hapi_rs::geometry::Geometry;
use hapi_rs::Result;

use ultraviolet::{Vec2, Vec3};

#[derive(Clone, Debug)]
#[repr(C)]
pub struct Vertex {
    position: Vec3,
    normal: Vec3,
    uv: Vec2,
}

#[derive(Default)]
pub struct MeshData {
    positions: Vec<f32>,
    face_counts: Vec<i32>,
    normals: Option<Vec<f32>>,
    color: Option<Vec<f32>>,
    uvs: Option<Vec<f32>>,
    pub vertex_array: Vec<Vertex>,
}

pub struct Asset {
    mesh: MeshData,
    node: Geometry,
    vao: Option<glow::VertexArray>,
    vbo: Option<glow::Buffer>,
}

impl MeshData {
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

        // TODO: Capacity
        let mut vertex_array = Vec::new();
        let mut offset = 0;

        for vertex_count_per_face in &face_counts {
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

                let mut vertex = Vertex {
                    position: pos_a,
                    normal: Vec3::zero(),
                    uv: Vec2::zero(),
                };

                if let Some(ref uvs) = uvs {
                    vertex.uv = Vec2::new(uvs[off0 * 3 + 0], uvs[off0 * 3 + 1]);
                }
                if let Some(ref normals) = normals {
                    vertex.normal = Vec3::new(
                        normals[off0 * 3 + 0],
                        normals[off0 * 3 + 1],
                        normals[off0 * 3 + 2],
                    );
                }
                vertex_array.push(vertex);

                let mut vertex = Vertex {
                    position: pos_b,
                    normal: Vec3::zero(),
                    uv: Vec2::zero(),
                };

                if let Some(ref uvs) = uvs {
                    vertex.uv = Vec2::new(uvs[off1 * 3 + 0], uvs[off1 * 3 + 1]);
                }
                if let Some(ref normals) = normals {
                    vertex.normal = Vec3::new(
                        normals[off1 * 3 + 0],
                        normals[off1 * 3 + 1],
                        normals[off1 * 3 + 2],
                    );
                }
                vertex_array.push(vertex);

                let mut vertex = Vertex {
                    position: pos_c,
                    normal: Vec3::zero(),
                    uv: Vec2::zero(),
                };

                if let Some(ref uvs) = uvs {
                    vertex.uv = Vec2::new(uvs[off2 * 3 + 0], uvs[off2 * 3 + 1]);
                }
                if let Some(ref normals) = normals {
                    vertex.normal = Vec3::new(
                        normals[off2 * 3 + 0],
                        normals[off2 * 3 + 1],
                        normals[off2 * 3 + 2],
                    );
                }
                vertex_array.push(vertex);

                // let uv_a = Vec2::new(uvs[off0 * 3 + 0], uv_attr[off0 * 3 + 1]);
                // let uv_b = Vec2::new(uvs[off1 * 3 + 0], uv_attr[off1 * 3 + 1]);
                // let uv_c = Vec2::new(uvs[off2 * 3 + 0], uv_attr[off2 * 3 + 1]);

                // vertex_array.extend_from_slice(&[
                //     Vertex {
                //         pos: pos_a,
                //         uv: uv_a,
                //     },
                //     Vertex {
                //         pos: pos_b,
                //         uv: uv_b,
                //     },
                //     Vertex {
                //         pos: pos_c,
                //         uv: uv_c,
                //     },
                // ]);
            }
        }

        Ok(Self {
            positions,
            face_counts,
            normals,
            color: None,
            uvs,
            vertex_array,
        })
    }

    pub fn new(
        positions: Vec<f32>,
        face_counts: Vec<i32>,
        normals: Option<Vec<f32>>,
        color: Option<Vec<f32>>,
        uvs: Option<Vec<f32>>,
    ) {
        // MeshData {
        //     positions,
        //     face_counts,
        //     normals,
        //     color,
        //     uvs,
        // }
        todo!()
    }
}

impl Asset {
    pub fn new(mesh: MeshData) -> Self {
        todo!()
        // Self {
        //     mesh: MeshData::default(),
        //     vao: None,
        //     vbo: None,
        // }
    }
}
