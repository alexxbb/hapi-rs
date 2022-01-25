use crate::ffi::{raw::HAPI_VolumeTileInfo, VolumeTileInfo};
use crate::node::HoudiniNode;
use crate::Result;

pub trait VolumeStorage: Sized + Copy {
    fn read_tile(
        node: &HoudiniNode,
        part: i32,
        fill: Self,
        values: &mut [Self],
        tile: &HAPI_VolumeTileInfo,
    ) -> Result<()>;

    fn write_tile(
        node: &HoudiniNode,
        part: i32,
        values: &[Self],
        tile: &HAPI_VolumeTileInfo,
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
        crate::ffi::get_volume_tile_int_data(&node, part, fill_value, values, info)
    }

    fn write_tile(
        node: &HoudiniNode,
        part: i32,
        values: &[Self],
        tile: &HAPI_VolumeTileInfo,
    ) -> Result<()> {
        crate::ffi::set_volume_tile_int_data(node, part, values, tile)
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
        crate::ffi::get_volume_tile_float_data(&node, part, fill_value, values, info)
    }

    fn write_tile(
        node: &HoudiniNode,
        part: i32,
        values: &[Self],
        tile: &HAPI_VolumeTileInfo,
    ) -> Result<()> {
        crate::ffi::set_volume_tile_float_data(node, part, values, tile)
    }
}

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
    let mut tile = VolumeTileInfo {
        inner: crate::ffi::get_volume_first_tile_info(&node, part)?,
    };
    let mut tile_num = 0;
    while tile.is_valid() {
        callback(Tile {
            info: &tile,
            size: tile_size,
            index: tile_num,
        });
        crate::ffi::get_volume_next_tile_info(&node, part, &mut tile.inner)?;
        tile_num += 1;
    }
    Ok(())
}

// pub(crate) fn read_volumes<T: VolumeStorage>(
//     node: &HoudiniNode,
//     info: &crate::ffi::VolumeInfo,
//     part: i32,
//     fill_value: T,
//     callback: impl Fn(&mut [T], usize, &crate::ffi::VolumeTileInfo),
// ) -> Result<()> {
//     let mut tile = VolumeTileInfo {
//         inner: crate::ffi::get_volume_first_tile_info(&node, part)?,
//     };
//     let tile_value_count = (info.tile_size().pow(3) * info.tuple_size()) as usize;
//     let mut values = vec![fill_value; tile_value_count];
//     let mut tile_num = 0;
//     while tile.is_valid() {
//         T::read_tile(
//             &node,
//             part,
//             fill_value,
//             values.as_mut_slice(),
//             &mut tile.inner,
//         )?;
//         callback(&mut values, tile_num, &tile);
//         crate::ffi::get_volume_next_tile_info(&node, part, &mut tile.inner)?;
//         tile_num += 1;
//     }
//     Ok(())
// }
