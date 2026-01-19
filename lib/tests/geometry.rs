use hapi_rs::geometry::extra::GeometryExtension;
use hapi_rs::{attribute::*, geometry::*};
mod utils;

use tempfile::NamedTempFile;
use utils::{HdaFile, create_triangle, with_session, with_test_geometry};

use crate::utils::with_session_asset;

#[test]
fn geometry_save_and_load_to_file() {
    with_session(|session| {
        let geo = create_triangle(&session)?;
        let tmp_file = NamedTempFile::new().expect("tempfile");
        geo.save_to_file(tmp_file.path().to_string_lossy().as_ref())
            .expect("save_to_file");
        geo.node.delete().unwrap();

        let geo = session.create_input_node("dummy", None).unwrap();
        geo.load_from_file(tmp_file.path().to_string_lossy().as_ref())
            .expect("load_from_file");
        geo.node.cook().unwrap();
        assert_eq!(geo.part_info(0).unwrap().point_count(), 3);
        geo.node.delete()
    })
    .unwrap()
}

#[test]
fn geometry_save_and_load_to_memory() {
    with_session(|session| {
        let src_geo = create_triangle(&session)?;
        let blob = src_geo
            .save_to_memory(GeoFormat::Geo)
            .expect("save_geo_to_memory");
        src_geo.node.delete().unwrap();

        let dest_geo = create_triangle(&session)?;
        dest_geo
            .load_from_memory(&blob, GeoFormat::Geo)
            .expect("load_from_memory");
        dest_geo.node.delete()
    })
    .unwrap()
}

#[test]
fn geometry_commit_and_revert() {
    with_session(|session| {
        let geo = create_triangle(&session)?;
        geo.commit().unwrap();
        geo.node.cook_blocking().unwrap();
        assert_eq!(geo.part_info(0).unwrap().point_count(), 3);
        geo.revert().unwrap();
        geo.node.cook_blocking().unwrap();
        assert_eq!(geo.part_info(0).unwrap().point_count(), 0);
        geo.node.delete()
    })
    .unwrap()
}

#[test]
fn geometry_elements() {
    with_test_geometry(|geo| {
        let part = geo.part_info(0).unwrap();
        // Cube
        let points = geo
            .get_element_count_by_owner(&part, AttributeOwner::Point)
            .unwrap();
        assert_eq!(points, 8);
        assert_eq!(points, part.point_count());
        let prims = geo
            .get_element_count_by_owner(&part, AttributeOwner::Prim)
            .unwrap();
        assert_eq!(prims, 6);
        assert_eq!(prims, part.face_count());
        let vtx = geo
            .get_element_count_by_owner(&part, AttributeOwner::Vertex)
            .unwrap();
        assert_eq!(vtx, 24);
        assert_eq!(vtx, part.vertex_count());
        let num_pt = geo
            .get_attribute_count_by_owner(&part, AttributeOwner::Point)
            .unwrap();
        assert_eq!(num_pt, 8);
        let num_pr = geo
            .get_attribute_count_by_owner(&part, AttributeOwner::Prim)
            .unwrap();
        assert_eq!(num_pr, 3);
        let num_det = geo
            .get_attribute_count_by_owner(&part, AttributeOwner::Detail)
            .unwrap();
        assert_eq!(num_det, 3);
        let pr_groups = geo.get_group_names(GroupType::Prim).unwrap();
        let pt_groups = geo.get_group_names(GroupType::Point).unwrap();
        #[allow(clippy::needless_collect)]
        {
            let pr_groups = pr_groups.iter_str().collect::<Vec<_>>();
            let pt_groups = pt_groups.iter_str().collect::<Vec<_>>();
            assert!(pr_groups.contains(&"group_A"));
            assert!(pt_groups.contains(&"group_B"));
            Ok(())
        }
    })
    .unwrap()
}

#[test]
fn geometry_partitions_report_counts() {
    with_test_geometry(|geo| {
        let info = geo.geo_info()?;
        let partitions = geo.partitions()?;
        assert_eq!(partitions.len() as i32, info.part_count());
        let part = partitions.first().expect("partition info");

        let vertex_list = geo.vertex_list(part)?;
        assert_eq!(vertex_list.len() as i32, part.vertex_count());

        let face_counts = geo.get_face_counts(part)?;
        assert_eq!(face_counts.len() as i32, part.face_count());
        assert_eq!(face_counts.into_iter().sum::<i32>(), part.vertex_count());

        let prim_groups = geo.get_group_names(GroupType::Prim)?;
        let prim_group_name = prim_groups.iter_str().next().expect("prim group name");
        let prim_membership = geo.get_group_membership(part, GroupType::Prim, prim_group_name)?;
        assert_eq!(
            prim_membership.len() as i32,
            part.element_count_by_group(GroupType::Prim)
        );

        let point_groups = geo.get_group_names(GroupType::Point)?;
        let point_group_name = point_groups.iter_str().next().expect("point group name");
        let point_membership =
            geo.get_group_membership(part, GroupType::Point, point_group_name)?;
        assert_eq!(
            point_membership.len() as i32,
            part.element_count_by_group(GroupType::Point)
        );
        Ok(())
    })
    .unwrap()
}

#[test]
fn geometry_delete_attribute() {
    with_session(|session| {
        let geo = create_triangle(&session)?;
        let id_attr = geo
            .get_attribute(0, AttributeOwner::Point, c"id")
            .unwrap()
            .unwrap();
        id_attr.delete(0).unwrap();
        geo.commit().unwrap();
        geo.node.cook_blocking().unwrap();
        assert!(
            geo.get_attribute(0, AttributeOwner::Point, c"id")?
                .is_none()
        );
        Ok(())
    })
    .unwrap()
}

#[test]
fn geometry_partitions() {
    with_session(|session| {
        let geo = create_triangle(&session)?;
        assert_eq!(geo.partitions().unwrap().len(), 1);
        assert!(geo.part_info(100).is_err());
        Ok(())
    })
    .unwrap()
}

#[test]
fn geometry_add_and_delete_group() {
    with_session(|session| {
        let mut geo = create_triangle(&session)?;
        geo.add_group(0, GroupType::Point, "test", Some(&[1, 1, 1]))
            .unwrap();
        geo.commit().unwrap();
        geo.node.cook_blocking().unwrap();
        geo.update().unwrap();
        assert_eq!(geo.geo_info().unwrap().point_group_count(), 1);
        assert_eq!(geo.group_count_by_type(GroupType::Point).unwrap(), 1);

        geo.delete_group(0, GroupType::Point, "test").unwrap();
        geo.commit().unwrap();
        geo.node.cook_blocking().unwrap();
        geo.update().unwrap();
        assert_eq!(geo.group_count_by_type(GroupType::Point).unwrap(), 0);
        geo.node.delete()
    })
    .unwrap()
}

#[test]
fn geometry_geo_info_updates_after_group_edits() {
    with_session(|session| {
        let mut geo = create_triangle(&session)?;
        let part = geo.part_info(0).expect("part_info");
        let baseline = geo.geo_info().expect("geo_info");
        let baseline_groups = baseline.point_group_count();

        let membership = vec![1; part.point_count() as usize];
        geo.add_group(
            part.part_id(),
            GroupType::Point,
            "lifecycle_group",
            Some(&membership),
        )
        .expect("add_group");
        geo.commit().expect("commit");
        geo.node.cook_blocking().expect("cook_blocking");
        geo.update().expect("update");
        let after_add = geo.geo_info().expect("geo_info");
        assert_eq!(after_add.point_group_count(), baseline_groups + 1);
        assert_eq!(after_add.part_count(), baseline.part_count());

        geo.delete_group(part.part_id(), GroupType::Point, "lifecycle_group")
            .expect("delete_group");
        geo.commit().expect("commit");
        geo.node.cook_blocking().expect("cook_blocking");
        geo.update().expect("update");
        let after_delete = geo.geo_info().expect("geo_info");
        assert_eq!(after_delete.point_group_count(), baseline_groups);
        assert_eq!(after_delete.part_count(), baseline.part_count());
        geo.node.delete()
    })
    .unwrap()
}

#[test]
fn geometry_basic_instancing() {
    with_session(|session| {
        session.load_asset_file(HdaFile::Geometry.path())?;
        let asset_node = session.create_node("Object/hapi_geo")?;
        asset_node.cook_blocking()?;
        let instancer = asset_node
            .get_child_by_path("instance")
            .unwrap()
            .expect("instance node");
        let geo = instancer.geometry().unwrap().expect("geometry");
        let opt =
            CookOptions::default().with_packed_prim_instancing_mode(PackedPrimInstancingMode::Flat);
        geo.node.cook_with_options(&opt, true).unwrap();
        let part = geo.part_info(0).unwrap();
        let ids = geo.get_instanced_part_ids(&part).unwrap();
        assert_eq!(ids.len(), 1);
        let names = geo
            .get_instance_part_groups_names(GroupType::Prim, ids[0])
            .unwrap();
        let names: Vec<String> = names.into_iter().collect();
        assert!(names.contains(&String::from("group_1")));
        assert!(names.contains(&String::from("group_6")));
        let transforms = geo
            .get_instance_part_transforms(&part, RSTOrder::Srt)
            .unwrap();
        assert_eq!(
            transforms.len() as i32,
            geo.part_info(0).unwrap().instance_count()
        );
        Ok(())
    })
    .unwrap()
}

#[test]
fn geometry_get_face_materials() {
    with_session(|session| {
        session.load_asset_file(HdaFile::Spaceship.path())?;
        let node = session.create_node("Object/spaceship").unwrap();
        node.cook_blocking().unwrap();
        let geo = node.geometry().expect("geometry").unwrap();
        let part = geo.part_info(0).unwrap();
        let mats = geo.get_materials(&part).expect("materials");
        assert!(matches!(mats, Some(Materials::Single(_))));
        Ok(())
    })
    .unwrap()
}

#[test]
fn geometry_create_input_curve() {
    with_session(|session| {
        let geo = session.create_input_curve_node("InputCurve", None).unwrap();
        let positions = &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        geo.set_input_curve_positions(0, positions).unwrap();
        let p = geo
            .get_position_attribute(&geo.part_info(0)?)
            .unwrap()
            .expect("position attribute");
        let coords = p.get(geo.part_info(0)?.part_id()).unwrap();
        assert_eq!(positions, coords.as_slice());
        Ok(())
    })
    .unwrap()
}

#[test]
fn geometry_multiple_input_curves() {
    with_session(|session| {
        let geo = session.create_input_node("InputCurves", None).unwrap();
        let points = vec![
            0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 2.0, 0.0, 0.0, 2.0, 1.0,
            0.0,
        ];
        let point_count = (points.len() / 3) as i32;
        let part_info = PartInfo::default()
            .with_part_type(PartType::Curve)
            .with_face_count(1)
            .with_vertex_count(point_count)
            .with_point_count(point_count);
        geo.set_part_info(&part_info).unwrap();

        let curve_info = CurveInfo::default()
            .with_curve_type(CurveType::Linear)
            .with_curve_count(3)
            .with_vertex_count(point_count)
            .with_order(4)
            .with_has_knots(false);

        geo.set_curve_info(0, &curve_info).unwrap();
        geo.set_curve_counts(part_info.part_id(), &[2, 2, 2])
            .unwrap();

        let p_info = AttributeInfo::default()
            .with_count(point_count)
            .with_tuple_size(3)
            .with_storage(StorageType::Float)
            .with_owner(AttributeOwner::Point);
        let p_attrib = geo.add_numeric_attribute::<f32>("P", 0, p_info).unwrap();
        p_attrib.set(0, &points).unwrap();
        geo.commit().unwrap();
        geo.node.cook_blocking().unwrap();
        let tmp_file = NamedTempFile::new().expect("tempfile");
        geo.save_to_file(tmp_file.path().to_string_lossy().as_ref())
            .expect("save_to_file");
        assert!(tmp_file.path().exists());
        Ok(())
    })
    .unwrap()
}

#[test]
fn geometry_read_write_volume() {
    with_session_asset(HdaFile::Volume, |lib| {
        let node = lib.try_create_first().expect("create_node");
        node.cook_blocking().expect("cook_blocking");
        let source = node.geometry().expect("geometry").unwrap();
        let source_part = source.part_info(0).unwrap();
        let vol_info = source.volume_info(0).unwrap();
        let dest_geo = node
            .session
            .create_input_node("volume_copy", None)
            .expect("create_input_node");
        dest_geo.set_part_info(&source_part).unwrap();
        dest_geo.set_volume_info(0, &vol_info).unwrap();

        source
            .foreach_volume_tile(0, &vol_info, |tile| {
                let mut values = vec![-1.0; tile.size];
                source
                    .read_volume_tile::<f32>(0, -1.0, tile.info, &mut values)
                    .unwrap();
                dest_geo
                    .write_volume_tile::<f32>(0, tile.info, &values)
                    .unwrap();
            })
            .unwrap();
        dest_geo.commit().unwrap();
        dest_geo.node.cook_blocking()?;
        Ok(())
    })
    .unwrap()
}

#[test]
fn geometry_extension_helpers_create_attributes() {
    with_session(|session| {
        let mut geo = session.create_input_node("extension_helpers", None)?;
        let part = PartInfo::default()
            .with_part_type(PartType::Mesh)
            .with_point_count(2)
            .with_vertex_count(0)
            .with_face_count(0);
        geo.set_part_info(&part)?;

        assert!(geo.get_position_attribute(&part)?.is_none());
        assert!(
            geo.get_color_attribute(&part, AttributeOwner::Point)?
                .is_none()
        );
        assert!(
            geo.get_normal_attribute(&part, AttributeOwner::Point)?
                .is_none()
        );

        let positions = geo.create_position_attribute(&part)?;
        positions.set(part.part_id(), &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0])?;

        let colors = geo.create_point_color_attribute(&part)?;
        colors.set(part.part_id(), &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0])?;

        geo.commit()?;
        geo.node.cook_blocking()?;
        geo.update()?;

        let fetched_positions = geo.get_position_attribute(&part)?.expect("position attr");
        assert_eq!(fetched_positions.info().tuple_size(), 3);
        assert_eq!(
            fetched_positions.get(part.part_id())?,
            vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0]
        );

        let fetched_colors = geo
            .get_color_attribute(&part, AttributeOwner::Point)?
            .expect("color attr");
        assert_eq!(fetched_colors.info().tuple_size(), 3);
        assert_eq!(
            fetched_colors.get(part.part_id())?,
            vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0]
        );

        assert!(
            geo.get_color_attribute(&part, AttributeOwner::Vertex)?
                .is_none()
        );
        assert!(
            geo.get_normal_attribute(&part, AttributeOwner::Point)?
                .is_none()
        );
        geo.node.delete()
    })
    .unwrap()
}
