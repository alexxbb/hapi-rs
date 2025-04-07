//! Volume and Heightfield APIs
use crate::Result;
use crate::ffi::{VolumeTileInfo, raw::HAPI_VolumeTileInfo};
use crate::node::HoudiniNode;

/// Volume primitive dimensions returned from [`crate::geometry::Geometry::volume_bounds()`]
#[derive(Debug, Clone, Default)]
pub struct VolumeBounds {
    pub x_min: f32,
    pub y_min: f32,
    pub z_min: f32,
    pub x_max: f32,
    pub y_max: f32,
    pub z_max: f32,
    pub x_center: f32,
    pub y_center: f32,
    pub z_center: f32,
}

pub trait VolumeStorage: Sized + Copy {
    fn read_tile(
        node: &HoudiniNode,
        part: i32,
        fill: Self,
        values: &mut [Self],
        tile: &HAPI_VolumeTileInfo,
    ) -> Result<()>;

    fn read_voxel(
        node: &HoudiniNode,
        part: i32,
        x: i32,
        y: i32,
        z: i32,
        values: &mut [Self],
    ) -> Result<()>;

    fn write_tile(
        node: &HoudiniNode,
        part: i32,
        values: &[Self],
        tile: &HAPI_VolumeTileInfo,
    ) -> Result<()>;

    fn write_voxel(
        node: &HoudiniNode,
        part: i32,
        x: i32,
        y: i32,
        z: i32,
        values: &[Self],
    ) -> Result<()>;
}

impl VolumeStorage for i32 {
    fn read_tile(
        node: &HoudiniNode,
        part: i32,
        fill_value: Self,
        values: &mut [Self],
        info: &HAPI_VolumeTileInfo,
    ) -> Result<()> {
        crate::ffi::get_volume_tile_int_data(node, part, fill_value, values, info)
    }

    fn read_voxel(
        node: &HoudiniNode,
        part: i32,
        x: i32,
        y: i32,
        z: i32,
        values: &mut [Self],
    ) -> Result<()> {
        crate::ffi::get_volume_voxel_int(node, part, x, y, z, values)
    }

    fn write_tile(
        node: &HoudiniNode,
        part: i32,
        values: &[Self],
        tile: &HAPI_VolumeTileInfo,
    ) -> Result<()> {
        crate::ffi::set_volume_tile_int_data(node, part, values, tile)
    }

    fn write_voxel(
        node: &HoudiniNode,
        part: i32,
        x: i32,
        y: i32,
        z: i32,
        values: &[Self],
    ) -> Result<()> {
        crate::ffi::set_volume_voxel_int(node, part, x, y, z, values)
    }
}

impl VolumeStorage for f32 {
    fn read_tile(
        node: &HoudiniNode,
        part: i32,
        fill_value: Self,
        values: &mut [Self],
        info: &HAPI_VolumeTileInfo,
    ) -> Result<()> {
        crate::ffi::get_volume_tile_float_data(node, part, fill_value, values, info)
    }

    fn read_voxel(
        node: &HoudiniNode,
        part: i32,
        x: i32,
        y: i32,
        z: i32,
        values: &mut [Self],
    ) -> Result<()> {
        crate::ffi::get_volume_voxel_float(node, part, x, y, z, values)
    }

    fn write_tile(
        node: &HoudiniNode,
        part: i32,
        values: &[Self],
        tile: &HAPI_VolumeTileInfo,
    ) -> Result<()> {
        crate::ffi::set_volume_tile_float_data(node, part, values, tile)
    }

    fn write_voxel(
        node: &HoudiniNode,
        part: i32,
        x: i32,
        y: i32,
        z: i32,
        values: &[Self],
    ) -> Result<()> {
        crate::ffi::set_volume_voxel_float(node, part, x, y, z, values)
    }
}

/// Represents a single tile in a volume.
/// Used with [`crate::geometry::Geometry::foreach_volume_tile`]
#[derive(Debug)]
pub struct Tile<'a> {
    pub info: &'a VolumeTileInfo,
    pub size: usize,
    pub index: i32,
}

pub(crate) fn iterate_tiles(
    node: &HoudiniNode,
    part: i32,
    tile_size: usize,
    callback: impl Fn(Tile),
) -> Result<()> {
    let mut tile = VolumeTileInfo(crate::ffi::get_volume_first_tile_info(node, part)?);
    let mut tile_num = 0;
    while tile.is_valid() {
        callback(Tile {
            info: &tile,
            size: tile_size,
            index: tile_num,
        });
        crate::ffi::get_volume_next_tile_info(node, part, &mut tile.0)?;
        tile_num += 1;
    }
    Ok(())
}
