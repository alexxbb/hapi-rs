use hapi_rs::geometry::extra::GeometryExtension;
use hapi_rs::{Result, attribute::*, geometry::*};
use pretty_assertions::assert_eq;
mod utils;

use tempfile::NamedTempFile;
use utils::{HdaFile, create_triangle, with_session, with_test_geometry};

#[test]
fn geometry_save_and_load_to_file() -> Result<()> {
    with_session(|session| {
        let geo = create_triangle(&session)?;
        let tmp_file = NamedTempFile::new()?;
        geo.save_to_file(tmp_file.path().to_string_lossy().as_ref())?;
        geo.node.delete()?;

        let geo = session.create_input_node("dummy", None)?;
        geo.load_from_file(tmp_file.path().to_string_lossy().as_ref())?;
        geo.node.cook()?;
        assert_eq!(geo.part_info(0)?.point_count(), 3);
        geo.node.delete()
    })
}

#[test]
fn geometry_save_and_load_to_memory() -> Result<()> {
    with_session(|session| {
        let src_geo = create_triangle(&session)?;
        let blob = src_geo.save_to_memory(GeoFormat::Geo)?;
        let bgeo = src_geo.save_to_memory(GeoFormat::Bgeo)?;
        let obj = src_geo.save_to_memory(GeoFormat::Obj)?;
        assert!(!bgeo.is_empty());
        assert!(!obj.is_empty());
        src_geo.node.delete()?;

        let dest_geo = create_triangle(&session)?;
        dest_geo.load_from_memory(&blob, GeoFormat::Geo)?;
        dest_geo.node.delete()
    })
}

#[test]
fn geometry_commit_and_revert() -> Result<()> {
    with_session(|session| {
        let geo = create_triangle(&session)?;
        geo.commit()?;
        geo.node.cook_blocking()?;
        assert_eq!(geo.part_info(0)?.point_count(), 3);
        geo.revert()?;
        geo.node.cook_blocking()?;
        assert_eq!(geo.part_info(0)?.point_count(), 0);
        geo.node.delete()
    })
}

#[test]
fn geometry_elements() -> Result<()> {
    with_test_geometry(|geo| {
        let part = geo.part_info(0)?;
        // Cube
        let points = geo.get_element_count_by_owner(&part, AttributeOwner::Point)?;
        assert_eq!(points, 8);
        assert_eq!(points, part.point_count());
        let prims = geo.get_element_count_by_owner(&part, AttributeOwner::Prim)?;
        assert_eq!(prims, 6);
        assert_eq!(prims, part.face_count());
        let vtx = geo.get_element_count_by_owner(&part, AttributeOwner::Vertex)?;
        assert_eq!(vtx, 24);
        assert_eq!(vtx, part.vertex_count());
        let num_pt = geo.get_attribute_count_by_owner(&part, AttributeOwner::Point)?;
        assert_eq!(num_pt, 8);
        let num_pr = geo.get_attribute_count_by_owner(&part, AttributeOwner::Prim)?;
        assert_eq!(num_pr, 3);
        let num_det = geo.get_attribute_count_by_owner(&part, AttributeOwner::Detail)?;
        assert_eq!(num_det, 3);
        let pr_groups = geo.get_group_names(GroupType::Prim)?;
        let pt_groups = geo.get_group_names(GroupType::Point)?;
        let edge_groups = geo.get_group_names(GroupType::Edge)?;
        let _ = geo.get_attribute_names(AttributeOwner::Vertex, &part)?;
        let _ = geo.get_attribute_names(AttributeOwner::Detail, &part)?;
        assert!(
            geo.get_attribute_names(AttributeOwner::Invalid, &part)
                .is_err()
        );
        #[allow(clippy::needless_collect)]
        {
            let pr_groups = pr_groups.iter_str().collect::<Vec<_>>();
            let pt_groups = pt_groups.iter_str().collect::<Vec<_>>();
            let edge_groups = edge_groups.iter_str().collect::<Vec<_>>();
            assert!(pr_groups.contains(&"group_A"));
            assert!(pt_groups.contains(&"group_B"));
            if let Some(edge_group) = edge_groups.first() {
                let _ = geo.get_edge_count_of_edge_group(edge_group, part.part_id())?;
            }
            Ok(())
        }
    })
}

#[test]
fn geometry_partitions_report_counts() -> Result<()> {
    with_test_geometry(|geo| {
        let info = geo.geo_info()?;
        let partitions = geo.partitions()?;
        assert_eq!(partitions.len() as i32, info.part_count());
        let part = partitions
            .first()
            .ok_or_else(|| hapi_rs::HapiError::Internal("partition info".into()))?;

        let vertex_list = geo.vertex_list(part)?;
        assert_eq!(vertex_list.len() as i32, part.vertex_count());

        let face_counts = geo.get_face_counts(part)?;
        assert_eq!(face_counts.len() as i32, part.face_count());
        assert_eq!(face_counts.into_iter().sum::<i32>(), part.vertex_count());

        let prim_groups = geo.get_group_names(GroupType::Prim)?;
        let prim_group_name = prim_groups
            .iter_str()
            .next()
            .ok_or_else(|| hapi_rs::HapiError::Internal("prim group name".into()))?;
        let prim_membership = geo.get_group_membership(part, GroupType::Prim, prim_group_name)?;
        assert_eq!(
            prim_membership.len() as i32,
            part.element_count_by_group(GroupType::Prim)
        );

        let point_groups = geo.get_group_names(GroupType::Point)?;
        let point_group_name = point_groups
            .iter_str()
            .next()
            .ok_or_else(|| hapi_rs::HapiError::Internal("point group name".into()))?;
        let point_membership =
            geo.get_group_membership(part, GroupType::Point, point_group_name)?;
        assert_eq!(
            point_membership.len() as i32,
            part.element_count_by_group(GroupType::Point)
        );

        let _ = geo.get_attribute_info(part.part_id(), AttributeOwner::Point, "P")?;
        let _ = geo.get_attribute_info(part.part_id(), AttributeOwner::Point, String::from("P"))?;
        let _ = geo.get_attribute_info(part.part_id(), AttributeOwner::Point, AttributeName::Uv)?;
        let _ = geo.get_attribute_info(
            part.part_id(),
            AttributeOwner::Point,
            AttributeName::TangentU,
        )?;
        let _ = geo.get_attribute_info(
            part.part_id(),
            AttributeOwner::Point,
            AttributeName::TangentV,
        )?;
        let _ =
            geo.get_attribute_info(part.part_id(), AttributeOwner::Point, AttributeName::Scale)?;
        let _ =
            geo.get_attribute_info(part.part_id(), AttributeOwner::Prim, AttributeName::Name)?;
        let _ = geo.get_attribute_info(
            part.part_id(),
            AttributeOwner::Point,
            AttributeName::from(c"custom_name"),
        )?;
        Ok(())
    })
}

#[test]
fn geometry_delete_attribute() -> Result<()> {
    with_session(|session| {
        let geo = create_triangle(&session)?;
        let id_attr = geo
            .get_attribute(0, AttributeOwner::Point, c"id")?
            .ok_or_else(|| hapi_rs::HapiError::Internal("id attribute".into()))?;
        id_attr.delete(0)?;
        geo.commit()?;
        geo.node.cook_blocking()?;
        assert!(
            geo.get_attribute(0, AttributeOwner::Point, c"id")?
                .is_none()
        );
        Ok(())
    })
}

#[test]
fn geometry_partitions() -> Result<()> {
    with_session(|session| {
        let geo = create_triangle(&session)?;
        assert_eq!(geo.partitions()?.len(), 1);
        assert!(geo.part_info(100).is_err());
        Ok(())
    })
}

#[test]
fn geometry_add_and_delete_group() -> Result<()> {
    with_session(|session| {
        let mut geo = create_triangle(&session)?;
        geo.add_group(0, GroupType::Point, "test", Some(&[1, 1, 1]))?;
        geo.add_group(0, GroupType::Point, "empty_group", None)?;
        geo.set_group_membership(0, GroupType::Point, "empty_group", &[1, 0, 1])?;
        geo.commit()?;
        geo.node.cook_blocking()?;
        geo.update()?;
        assert_eq!(geo.geo_info()?.point_group_count(), 2);
        assert_eq!(geo.group_count_by_type(GroupType::Point)?, 2);

        geo.delete_group(0, GroupType::Point, "test")?;
        geo.delete_group(0, GroupType::Point, "empty_group")?;
        geo.commit()?;
        geo.node.cook_blocking()?;
        geo.update()?;
        assert_eq!(geo.group_count_by_type(GroupType::Point)?, 0);
        geo.node.delete()
    })
}

#[test]
fn geometry_geo_info_updates_after_group_edits() -> Result<()> {
    with_session(|session| {
        let mut geo = create_triangle(&session)?;
        let part = geo.part_info(0)?;
        let baseline = geo.geo_info()?;
        let baseline_groups = baseline.point_group_count();

        let membership = vec![1; part.point_count() as usize];
        geo.add_group(
            part.part_id(),
            GroupType::Point,
            "lifecycle_group",
            Some(&membership),
        )?;
        geo.commit()?;
        geo.node.cook_blocking()?;
        geo.update()?;
        let after_add = geo.geo_info()?;
        assert_eq!(after_add.point_group_count(), baseline_groups + 1);
        assert_eq!(after_add.part_count(), baseline.part_count());

        geo.delete_group(part.part_id(), GroupType::Point, "lifecycle_group")?;
        geo.commit()?;
        geo.node.cook_blocking()?;
        geo.update()?;
        let after_delete = geo.geo_info()?;
        assert_eq!(after_delete.point_group_count(), baseline_groups);
        assert_eq!(after_delete.part_count(), baseline.part_count());
        geo.node.delete()
    })
}

#[test]
fn geometry_basic_instancing() -> Result<()> {
    with_session(|session| {
        session.load_asset_file(HdaFile::Geometry.path())?;
        let asset_node = session.create_node("Object/hapi_geo")?;
        asset_node.cook_blocking()?;
        let instancer = asset_node
            .get_child_by_path("instance")?
            .ok_or_else(|| hapi_rs::HapiError::Internal("instance node".into()))?;
        let geo = instancer
            .geometry()?
            .ok_or_else(|| hapi_rs::HapiError::Internal("geometry".into()))?;
        let opt =
            CookOptions::default().with_packed_prim_instancing_mode(PackedPrimInstancingMode::Flat);
        geo.node.cook_with_options(&opt, true)?;
        let part = geo.part_info(0)?;
        let ids = geo.get_instanced_part_ids(&part)?;
        assert_eq!(ids.len(), 1);
        let names = geo.get_instance_part_groups_names(GroupType::Prim, ids[0])?;
        let names: Vec<String> = names.into_iter().collect();
        assert!(names.contains(&String::from("group_1")));
        assert!(names.contains(&String::from("group_6")));
        let transforms = geo.get_instance_part_transforms(&part, RSTOrder::Srt)?;
        assert_eq!(transforms.len() as i32, geo.part_info(0)?.instance_count());
        Ok(())
    })
}

#[test]
fn geometry_get_face_materials() -> Result<()> {
    with_session(|session| {
        session.load_asset_file(HdaFile::Spaceship.path())?;
        let node = session.create_node("Object/spaceship")?;
        node.cook_blocking()?;
        let geo = node
            .geometry()?
            .ok_or_else(|| hapi_rs::HapiError::Internal("geometry".into()))?;
        let part = geo.part_info(0)?;
        let mats = geo
            .get_materials(&part)?
            .ok_or_else(|| hapi_rs::HapiError::Internal("materials".into()))?;
        assert!(matches!(mats, Materials::Single(_)));
        Ok(())
    })
}

#[test]
fn geometry_create_input_curve() -> Result<()> {
    with_session(|session| {
        let geo = session.create_input_curve_node("InputCurve", None)?;
        let info = InputCurveInfo::default()
            .with_curve_type(CurveType::Linear)
            .with_order(2);
        geo.set_input_curve_info(0, &info)?;
        let positions = &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        geo.set_input_curve_positions(0, positions)?;
        geo.set_input_curve_transform(
            0,
            &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            &[1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
        )?;
        let _ = geo.get_input_curve_info(0)?;
        let part_info = geo.part_info(0)?;
        let p = geo
            .get_position_attribute(&part_info)?
            .ok_or_else(|| hapi_rs::HapiError::Internal("position attribute".into()))?;
        let coords = p.get(part_info.part_id())?;
        dbg!(&coords);
        assert_eq!(positions, coords.as_slice());
        Ok(())
    })
}

#[test]
fn geometry_attribute_wrappers_for_storage_variants() -> Result<()> {
    with_session(|session| {
        let geo = utils::create_single_point_geo(&session)?;
        let part = geo.part_info(0)?;

        let scalar_info = |storage| {
            AttributeInfo::default()
                .with_count(part.point_count())
                .with_tuple_size(1)
                .with_owner(AttributeOwner::Point)
                .with_storage(storage)
        };
        let array_info = |storage| {
            AttributeInfo::default()
                .with_count(part.point_count())
                .with_tuple_size(1)
                .with_owner(AttributeOwner::Point)
                .with_storage(storage)
                .with_total_array_elements(1)
        };

        geo.add_numeric_attribute::<i64>("i64_attr", 0, scalar_info(StorageType::Int64))?
            .set(0, &[1])?;
        geo.add_numeric_attribute::<f64>("f64_attr", 0, scalar_info(StorageType::Float64))?
            .set(0, &[1.0])?;
        geo.add_numeric_attribute::<u8>("u8_attr", 0, scalar_info(StorageType::Uint8))?
            .set(0, &[1])?;
        geo.add_numeric_attribute::<i8>("i8_attr", 0, scalar_info(StorageType::Int8))?
            .set(0, &[1])?;
        geo.add_numeric_attribute::<i16>("i16_attr", 0, scalar_info(StorageType::Int16))?
            .set(0, &[1])?;
        geo.add_numeric_array_attribute::<i64>("i64_array", 0, array_info(StorageType::Int64Array))?
            .set(0, &DataArray::new(&[1i64], &[1]))?;
        geo.add_numeric_array_attribute::<f64>(
            "f64_array",
            0,
            array_info(StorageType::Float64Array),
        )?
        .set(0, &DataArray::new(&[1.0f64], &[1]))?;
        geo.add_numeric_array_attribute::<u8>("u8_array", 0, array_info(StorageType::Uint8Array))?
            .set(0, &DataArray::new(&[1u8], &[1]))?;
        geo.add_numeric_array_attribute::<i8>("i8_array", 0, array_info(StorageType::Int8Array))?
            .set(0, &DataArray::new(&[1i8], &[1]))?;
        geo.add_numeric_array_attribute::<i16>("i16_array", 0, array_info(StorageType::Int16Array))?
            .set(0, &DataArray::new(&[1i16], &[1]))?;
        geo.add_dictionary_attribute("dict_attr", 0, scalar_info(StorageType::Dictionary))?
            .set(0, &[c"FOO=123"])?;
        geo.add_dictionary_array_attribute(
            "dict_array",
            0,
            array_info(StorageType::DictionaryArray),
        )?
        .set(0, &[c"FOO=123"], &[1])?;
        geo.commit()?;
        geo.node.cook_blocking()?;

        for name in [
            c"i64_attr",
            c"f64_attr",
            c"u8_attr",
            c"i8_attr",
            c"i16_attr",
            c"i64_array",
            c"f64_array",
            c"u8_array",
            c"i8_array",
            c"i16_array",
            c"dict_attr",
            c"dict_array",
        ] {
            assert!(geo.get_attribute(0, AttributeOwner::Point, name)?.is_some());
        }
        Ok(())
    })
}

#[test]
fn geometry_curve_knots_and_orders() -> Result<()> {
    with_session(|session| {
        let geo = session.create_input_node("curve_knots_and_orders", None)?;
        let points = [
            0.0, 0.0, 0.0, //
            1.0, 0.0, 0.0, //
            2.0, 0.0, 0.0, //
            3.0, 0.0, 0.0,
        ];
        let point_count = (points.len() / 3) as i32;
        let part_info = PartInfo::default()
            .with_part_type(PartType::Curve)
            .with_face_count(1)
            .with_vertex_count(point_count)
            .with_point_count(point_count);
        geo.set_part_info(&part_info)?;

        let curve_info = CurveInfo::default()
            .with_curve_type(CurveType::Nurbs)
            .with_curve_count(1)
            .with_vertex_count(point_count)
            .with_knot_count(7)
            .with_order(3)
            .with_has_knots(true);
        geo.set_curve_info(0, &curve_info)?;
        geo.set_curve_counts(0, &[point_count])?;
        geo.set_curve_knots(0, &[0.0, 0.0, 0.0, 0.5, 1.0, 1.0, 1.0])?;

        let p_info = AttributeInfo::default()
            .with_count(point_count)
            .with_tuple_size(3)
            .with_storage(StorageType::Float)
            .with_owner(AttributeOwner::Point);
        geo.add_numeric_attribute::<f32>("P", 0, p_info)?
            .set(0, &points)?;
        geo.commit()?;
        geo.node.cook_blocking()?;

        assert_eq!(geo.curve_orders(0, 0, 1)?, vec![3]);
        assert_eq!(geo.curve_knots(0, 0, 7)?.len(), 7);
        Ok(())
    })
}

#[test]
#[ignore = "reason: this test is flaky"]
fn geometry_heightfield_helpers() -> Result<()> {
    with_session(|session| {
        let geo = session.create_input_node("heightfield_helpers", None)?;
        let nodes =
            geo.create_heightfield_input(None, "height", 4, 4, 1.0, HeightFieldSampling::Center)?;
        let height_geo = nodes
            .height
            .geometry()?
            .ok_or_else(|| hapi_rs::HapiError::Internal("height geometry".into()))?;
        let data = vec![1.0; 16];
        nodes.height.cook_blocking()?;
        let volume_info = height_geo.volume_info(0)?;
        height_geo.set_volume_info(0, &volume_info)?;
        height_geo.set_heightfield_data(0, "height", &data)?;
        nodes.height.cook_blocking()?;
        let read_back = height_geo.get_heightfield_data(0, &volume_info)?;
        assert_eq!(read_back, data);

        let volume_node =
            geo.create_heightfield_input_volume(nodes.heightfield.handle, "mask", 4, 4, 1.0)?;
        volume_node.cook_blocking()?;
        assert!(volume_node.geometry()?.is_some());
        Ok(())
    })
}

#[test]
fn geometry_multiple_input_curves() -> Result<()> {
    with_session(|session| {
        let geo = session.create_input_node("InputCurves", None)?;
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
        geo.set_part_info(&part_info)?;

        let curve_info = CurveInfo::default()
            .with_curve_type(CurveType::Linear)
            .with_curve_count(3)
            .with_vertex_count(point_count)
            .with_order(4)
            .with_has_knots(false);

        geo.set_curve_info(0, &curve_info)?;
        geo.set_curve_counts(part_info.part_id(), &[2, 2, 2])?;

        let p_info = AttributeInfo::default()
            .with_count(point_count)
            .with_tuple_size(3)
            .with_storage(StorageType::Float)
            .with_owner(AttributeOwner::Point);
        let p_attrib = geo.add_numeric_attribute::<f32>("P", 0, p_info)?;
        p_attrib.set(0, &points)?;
        geo.commit()?;
        geo.node.cook_blocking()?;
        let info = geo.curve_info(0)?;
        assert_eq!(info.curve_count(), 3);
        assert_eq!(geo.curve_counts(0, 0, 3)?, vec![2, 2, 2]);
        let tmp_file = NamedTempFile::new()?;
        geo.save_to_file(tmp_file.path().to_string_lossy().as_ref())?;
        assert!(tmp_file.path().exists());
        Ok(())
    })
}

#[test]
fn geometry_extension_helpers_create_attributes() -> Result<()> {
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

        let fetched_positions = geo
            .get_position_attribute(&part)?
            .ok_or_else(|| hapi_rs::HapiError::Internal("position attr".into()))?;
        assert_eq!(fetched_positions.info().tuple_size(), 3);
        assert_eq!(
            fetched_positions.get(part.part_id())?,
            vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0]
        );

        let fetched_colors = geo
            .get_color_attribute(&part, AttributeOwner::Point)?
            .ok_or_else(|| hapi_rs::HapiError::Internal("color attr".into()))?;
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
}
