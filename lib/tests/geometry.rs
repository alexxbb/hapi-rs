use hapi_rs::{attribute::*, geometry::*};
mod _utils;

use _utils::{create_triangle, with_session};

#[test]
fn geometry_save_and_load_to_file() {
    with_session(|session| {
        let geo = create_triangle(&session)?;
        let tmp_file = std::env::temp_dir().join("triangle.geo");
        geo.save_to_file(&tmp_file.to_string_lossy())
            .expect("save_to_file");
        geo.node.delete().unwrap();

        let geo = session.create_input_node("dummy", None).unwrap();
        geo.load_from_file(&tmp_file.to_string_lossy())
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
    with_session(|session| {
        let node = session.create_node("Object/hapi_geo").unwrap();
        node.cook_blocking().unwrap();
        let geo = node.geometry().unwrap().expect("Geometry");
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
        assert_eq!(num_det, 4);
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
fn geometry_basic_instancing() {
    with_session(|session| {
        let node = session.create_node("Object/hapi_geo").unwrap();
        node.cook_blocking().unwrap();
        let opt =
            CookOptions::default().with_packed_prim_instancing_mode(PackedPrimInstancingMode::Flat);
        node.cook_with_options(&opt, true).unwrap();
        let instancer = node
            .get_child_by_path("instance")
            .unwrap()
            .expect("instance node");
        let geo = instancer.geometry().unwrap().expect("geometry");
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
        let p = geo.get_position_attribute(0).unwrap();
        let coords = p.get(0).unwrap();
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
        geo.save_to_file("c:/Temp/curve.geo")
    })
    .unwrap()
}

#[test]
fn geometry_read_write_volume() {
    with_session(|session| {
        let node = session.create_node("Object/hapi_vol").unwrap();
        node.cook_blocking().unwrap();
        let source = node.geometry().unwrap().unwrap();
        let source_part = source.part_info(0).unwrap();
        let vol_info = source.volume_info(0).unwrap();
        let dest_geo = session.create_input_node("volume_copy", None).unwrap();
        dest_geo.node.cook_blocking().unwrap();
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
