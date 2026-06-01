use hapi_rs::volume::VolumeBounds;
use hapi_rs::{Result, geometry::*};
use pretty_assertions::assert_eq;

mod utils;

use utils::{HdaFile, with_session_asset};

#[test]
fn volume_read_write_float() -> Result<()> {
    with_session_asset(HdaFile::Volume, |lib| {
        let node = lib.try_create_first()?;
        node.cook_blocking()?;
        let source = node
            .geometry()?
            .ok_or_else(|| hapi_rs::HapiError::Internal("geometry".into()))?;
        let part = source.part_info(0)?;
        let part_id = part.part_id();
        let vol_info = source.volume_info(part_id)?;
        let _ = source.get_volume_visual_info(part_id)?;

        let dest_geo = node.session.create_input_node("volume_copy", None)?;
        dest_geo.set_part_info(&part)?;
        dest_geo.set_volume_info(part_id, &vol_info)?;

        source.foreach_volume_tile(part_id, &vol_info, |tile| {
            let mut values = vec![-1.0; tile.size];
            source
                .read_volume_tile::<f32>(part_id, -1.0, tile.info, &mut values)
                .unwrap();
            dest_geo
                .write_volume_tile::<f32>(part_id, tile.info, &values)
                .unwrap();
        })?;

        let mut voxel = vec![0.0f32];
        source.read_volume_voxel(part_id, 0, 0, 0, &mut voxel)?;
        dest_geo.write_volume_voxel(part_id, 0, 0, 0, &voxel)?;
        dest_geo.commit()?;
        dest_geo.node.cook_blocking()?;
        dest_geo.save_to_file("/tmp/volume.bgeo")?;

        let mut read_back = vec![0.0f32];
        let _ = dest_geo.volume_info(part_id)?;
        dest_geo.read_volume_voxel(part_id, 0, 0, 0, &mut read_back)?;
        assert_eq!(voxel, read_back);

        Ok(())
    })
}

#[test]
fn volume_create_input_part() -> Result<()> {
    with_session_asset(HdaFile::Volume, |lib| {
        let template = lib.try_create_first()?;
        template.cook_blocking()?;
        let template_geo = template
            .geometry()?
            .ok_or_else(|| hapi_rs::HapiError::Internal("geometry".into()))?;
        let vol_info = template_geo.volume_info(0)?;

        let geo = template.session.create_input_node("volume_input", None)?;
        geo.set_part_info(&PartInfo::default().with_part_type(PartType::Volume))?;
        geo.set_volume_info(0, &vol_info)?;
        geo.commit()?;
        geo.node.cook_blocking()?;

        assert_eq!(geo.part_info(0)?.part_type(), PartType::Volume);
        assert_eq!(geo.volume_info(0)?.storage(), vol_info.storage());
        Ok(())
    })
}

#[allow(unused)]
fn assert_bounds_match(source: &VolumeBounds, dest: &VolumeBounds) {
    assert_eq!(source.x_min, dest.x_min);
    assert_eq!(source.y_min, dest.y_min);
    assert_eq!(source.z_min, dest.z_min);
    assert_eq!(source.x_max, dest.x_max);
    assert_eq!(source.y_max, dest.y_max);
    assert_eq!(source.z_max, dest.z_max);
}
