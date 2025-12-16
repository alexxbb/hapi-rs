use bevy::asset::RenderAssetUsages;
use bevy::prelude::Mesh;
use bevy::render::mesh::PrimitiveTopology;
use hapi_rs::Result;
use hapi_rs::attribute::NumericAttr;
use hapi_rs::geometry::{AttributeName, AttributeOwner, Geometry, extra::GeometryExtension};

pub enum NormalAttribute {
    Point(Vec<f32>),
    Vertex(Vec<f32>),
}

pub enum ColorAttribute {
    Point(Vec<f32>),
    Vertex(Vec<f32>),
}

pub struct HoudiniMeshData {
    pub vertex_list: Vec<i32>,
    pub face_counts: Vec<i32>,
    pub positions: Option<Vec<f32>>,
    pub normals: Option<NormalAttribute>,
    pub colors: Option<ColorAttribute>,
    pub uvs: Option<Vec<f32>>,
}

fn get_houdini_geometry_color(geometry: &Geometry) -> Result<Option<ColorAttribute>> {
    let part = geometry.part_info(0)?;
    let mut is_point_attr = false;
    let mut cd_attr = geometry.get_color_attribute(&part, AttributeOwner::Vertex)?;
    if cd_attr.is_none() {
        is_point_attr = true;
        cd_attr = geometry.get_color_attribute(&part, AttributeOwner::Point)?;
    }
    match cd_attr {
        Some(cd_attr) => {
            let values = cd_attr.get(0)?;
            Ok(Some(match is_point_attr {
                true => ColorAttribute::Point(values),
                false => ColorAttribute::Vertex(values),
            }))
        }
        None => Ok(None),
    }
}

fn get_houdini_geometry_normals(geometry: &Geometry) -> Result<Option<NormalAttribute>> {
    let part = geometry.part_info(0)?;
    let mut is_point_attr = false;
    let mut n_attr = geometry.get_normal_attribute(&part, AttributeOwner::Vertex)?;
    if n_attr.is_none() {
        is_point_attr = true;
        n_attr = geometry.get_normal_attribute(&part, AttributeOwner::Point)?;
    }
    match n_attr {
        Some(n_attr) => {
            let values = n_attr.get(0)?;
            Ok(Some(match is_point_attr {
                true => NormalAttribute::Point(values),
                false => NormalAttribute::Vertex(values),
            }))
        }
        None => Ok(None),
    }
}

#[derive(Debug, Default)]
pub struct HoudiniGeometryDataBuilder {
    positions: bool,
    normals: bool,
    colors: bool,
    uv: bool,
}

impl HoudiniGeometryDataBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_positions(mut self) -> HoudiniGeometryDataBuilder {
        self.positions = true;
        self
    }

    pub fn with_normals(mut self) -> HoudiniGeometryDataBuilder {
        self.normals = true;
        self
    }

    pub fn with_uv(mut self) -> HoudiniGeometryDataBuilder {
        self.uv = true;
        self
    }

    pub fn with_color(mut self) -> HoudiniGeometryDataBuilder {
        self.colors = true;
        self
    }
    pub fn build(self, geometry: &Geometry) -> Result<HoudiniMeshData> {
        get_houdini_geometry_data_arrays(
            geometry,
            self.positions,
            self.normals,
            self.colors,
            self.uv,
        )
    }
}

fn get_houdini_geometry_data_arrays(
    geometry: &Geometry,
    build_positions: bool,
    build_normals: bool,
    build_colors: bool,
    build_uv: bool,
) -> Result<HoudiniMeshData> {
    let part = geometry.part_info(0)?;
    let vertex_list = geometry.vertex_list(&part)?;
    let face_counts = geometry.get_face_counts(&part)?;
    let mut positions = None;
    let mut normals = None;
    let mut colors = None;
    let mut uvs = None;
    if build_positions {
        positions = Some(
            geometry
                .get_position_attribute(&part)?
                .expect("Position attribute must exist")
                .get(part.part_id())?,
        )
    }
    if build_normals {
        normals = get_houdini_geometry_normals(geometry)?;
    }

    if build_colors {
        colors = get_houdini_geometry_color(geometry)?;
    }

    if build_uv {
        uvs = match geometry.get_attribute(
            part.part_id(),
            AttributeOwner::Vertex,
            AttributeName::Uv,
        )? {
            None => None,
            Some(attr) => Some(
                attr.downcast::<NumericAttr<f32>>()
                    .expect("UV is NumericAttribute")
                    .get(part.part_id())?,
            ),
        };
    }

    Ok(HoudiniMeshData {
        vertex_list,
        face_counts,
        positions,
        normals,
        colors,
        uvs,
    })
}

pub fn vertex_deform(geometry: &Geometry) -> Result<BevyMeshData> {
    let mesh_data = HoudiniGeometryDataBuilder::new()
        .with_positions()
        .with_normals()
        .build(&geometry)?;

    Ok(convert_vertex_data(mesh_data))
}

struct Offset {
    global_vertex_offset: (usize, usize, usize),
    triangle_vertex_offset: (usize, usize, usize),
}

fn iterate_buffer_offsets(vertex_list: &[i32], face_counts: &[i32], mut func: impl FnMut(Offset)) {
    let mut offset = 0usize;

    for vertex_per_face in face_counts {
        let vertex_per_face = *vertex_per_face as usize;
        let num_tris = vertex_per_face - 2;

        for tr in 0..num_tris {
            let off0 = offset + tr + 2;
            let off1 = offset + tr + 1;
            let off2 = offset + 0;

            let point_0_index = vertex_list[off0] as usize;
            let point_1_index = vertex_list[off1] as usize;
            let point_2_index = vertex_list[off2] as usize;
            let offset = Offset {
                global_vertex_offset: (off0, off1, off2),
                triangle_vertex_offset: (point_0_index, point_1_index, point_2_index),
            };
            func(offset);
        }
        offset += vertex_per_face;
    }
}

pub struct BevyMeshData {
    pub vertices: Option<Vec<[f32; 3]>>,
    pub normals: Option<Vec<[f32; 3]>>,
    pub colors: Option<Vec<[f32; 4]>>,
    pub uvs: Option<Vec<[f32; 2]>>,
}

pub fn convert_vertex_data(data: HoudiniMeshData) -> BevyMeshData {
    let HoudiniMeshData {
        vertex_list,
        face_counts,
        positions,
        normals,
        colors,
        uvs,
    } = data;

    let num_vertices = face_counts.iter().sum::<i32>() as usize;
    let mut out_vertices = positions
        .is_some()
        .then(|| Vec::with_capacity(num_vertices));
    let mut out_normals = normals.is_some().then(|| Vec::with_capacity(num_vertices));
    let mut out_colors = colors.is_some().then(|| Vec::with_capacity(num_vertices));
    let mut out_uvs = uvs.is_some().then(|| Vec::with_capacity(num_vertices));

    iterate_buffer_offsets(&vertex_list, &face_counts, |offset| {
        let Offset {
            global_vertex_offset,
            triangle_vertex_offset,
            ..
        } = offset;

        if let (Some(positions), Some(out_vertices)) = (&positions, &mut out_vertices) {
            out_vertices.push([
                positions[triangle_vertex_offset.0 * 3 + 0],
                positions[triangle_vertex_offset.0 * 3 + 1],
                positions[triangle_vertex_offset.0 * 3 + 2],
            ]);
            out_vertices.push([
                positions[triangle_vertex_offset.1 * 3 + 0],
                positions[triangle_vertex_offset.1 * 3 + 1],
                positions[triangle_vertex_offset.1 * 3 + 2],
            ]);
            out_vertices.push([
                positions[triangle_vertex_offset.2 * 3 + 0],
                positions[triangle_vertex_offset.2 * 3 + 1],
                positions[triangle_vertex_offset.2 * 3 + 2],
            ]);
        }

        if let (Some(normals_attr), Some(out_normals)) = (&normals, &mut out_normals) {
            let (data, idx_0, idx_1, idx_2) = match normals_attr {
                NormalAttribute::Point(data) => (
                    data,
                    triangle_vertex_offset.0,
                    triangle_vertex_offset.1,
                    triangle_vertex_offset.2,
                ),
                NormalAttribute::Vertex(data) => (
                    data,
                    global_vertex_offset.0,
                    global_vertex_offset.1,
                    global_vertex_offset.2,
                ),
            };
            out_normals.push([
                data[idx_0 * 3 + 0],
                data[idx_0 * 3 + 1],
                data[idx_0 * 3 + 2],
            ]);

            out_normals.push([
                data[idx_1 * 3 + 0],
                data[idx_1 * 3 + 1],
                data[idx_1 * 3 + 2],
            ]);

            out_normals.push([
                data[idx_2 * 3 + 0],
                data[idx_2 * 3 + 1],
                data[idx_2 * 3 + 2],
            ]);
        }

        if let (Some(colors_attr), Some(out_colors)) = (&colors, &mut out_colors) {
            let (data, idx_0, idx_1, idx_2) = match colors_attr {
                ColorAttribute::Point(data) => (
                    data,
                    triangle_vertex_offset.0,
                    triangle_vertex_offset.1,
                    triangle_vertex_offset.2,
                ),
                ColorAttribute::Vertex(data) => (
                    data,
                    global_vertex_offset.0,
                    global_vertex_offset.1,
                    global_vertex_offset.2,
                ),
            };

            out_colors.push([
                data[idx_0 * 3 + 0],
                data[idx_0 * 3 + 1],
                data[idx_0 * 3 + 2],
                0.,
            ]);

            out_colors.push([
                data[idx_1 * 3 + 0],
                data[idx_1 * 3 + 1],
                data[idx_1 * 3 + 2],
                0.,
            ]);

            out_colors.push([
                data[idx_2 * 3 + 0],
                data[idx_2 * 3 + 1],
                data[idx_2 * 3 + 2],
                0.,
            ]);
        }

        if let (Some(uvs), Some(out_uvs)) = (&uvs, &mut out_uvs) {
            out_uvs.extend([
                [
                    uvs[global_vertex_offset.0 * 3 + 0],
                    1.0 - uvs[global_vertex_offset.0 * 3 + 1],
                ],
                [
                    uvs[global_vertex_offset.1 * 3 + 0],
                    1.0 - uvs[global_vertex_offset.1 * 3 + 1],
                ],
                [
                    uvs[global_vertex_offset.2 * 3 + 0],
                    1.0 - uvs[global_vertex_offset.2 * 3 + 1],
                ],
            ]);
        }
    });

    BevyMeshData {
        vertices: out_vertices,
        normals: out_normals,
        colors: out_colors,
        uvs: out_uvs,
    }
}

pub fn create_bevy_mesh_from_houdini(geometry: &Geometry) -> Result<Mesh> {
    let mesh_data = HoudiniGeometryDataBuilder::new()
        .with_positions()
        .with_normals()
        .with_color()
        .with_uv()
        .build(&geometry)?;

    let bevy_mesh = convert_vertex_data(mesh_data);

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    if let Some(position) = bevy_mesh.vertices {
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, position);
    }
    if let Some(colors) = bevy_mesh.colors {
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }

    if let Some(uv) = bevy_mesh.uvs {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    }

    if let Some(normals) = bevy_mesh.normals {
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        if mesh.generate_tangents().is_err() {
            eprintln!("Could not compute mesh tangents");
        }
    } else {
        // smooth normals can only be computed for an indexed mesh
        println!("Computing flat normals");
        mesh.compute_flat_normals();
    }

    Ok(mesh)
}
