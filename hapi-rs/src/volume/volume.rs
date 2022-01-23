use crate::ffi::raw::HAPI_VolumeTileInfo;
use crate::node::HoudiniNode;
use crate::Result;

pub(crate) trait ValueType: Sized + Copy {
    fn get_value_tile(
        node: &HoudiniNode,
        part: i32,
        fill: Self,
        values: &mut [Self],
        tile: &mut HAPI_VolumeTileInfo,
    ) -> Result<()>;

    fn set_value_tile(
        node: &HoudiniNode,
        part: i32,
        fill: Self,
        values: &mut [Self],
        tile: &mut HAPI_VolumeTileInfo,
    ) -> Result<()>;
}

impl ValueType for i32 {
    fn get_value_tile(
        node: &HoudiniNode,
        part: i32,
        fill: Self,
        values: &mut [Self],
        tile: &mut HAPI_VolumeTileInfo,
    ) -> Result<()> {
        crate::ffi::get_volume_tile_int_data(&node, part, fill, values, tile)
    }

    fn set_value_tile(
        node: &HoudiniNode,
        part: i32,
        fill: Self,
        values: &mut [Self],
        tile: &mut HAPI_VolumeTileInfo,
    ) -> Result<()> {
        todo!()
    }
}

impl ValueType for f32 {
    fn get_value_tile(
        node: &HoudiniNode,
        part: i32,
        fill: Self,
        values: &mut [Self],
        tile: &mut HAPI_VolumeTileInfo,
    ) -> Result<()> {
        crate::ffi::get_volume_tile_float_data(&node, part, fill, values, tile)
    }

    fn set_value_tile(
        node: &HoudiniNode,
        part: i32,
        fill: Self,
        values: &mut [Self],
        tile: &mut HAPI_VolumeTileInfo,
    ) -> Result<()> {
        todo!()
    }
}

pub(crate) fn read_volume<T: ValueType>(
    node: &HoudiniNode,
    info: &crate::ffi::VolumeInfo,
    part: i32,
    fill_value: T,
    callback: impl Fn(&mut [T], usize),
) -> Result<()> {
    let mut tile = crate::ffi::get_volume_first_tile_info(&node, part)?;
    let tile_value_count = (info.tile_size().pow(3) * info.tuple_size()) as usize;
    let mut values = vec![fill_value; tile_value_count];
    let mut tile_num = 0;
    while tile.isValid > 0 {
        T::get_value_tile(&node, part, fill_value, values.as_mut_slice(), &mut tile)?;
        callback(&mut values, tile_num);
        crate::ffi::get_volume_next_tile_info(&node, part, &mut tile)?;
        tile_num += 1;
    }

    Ok(())
}
