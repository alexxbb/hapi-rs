use once_cell::unsync::Lazy;

use hapi_rs::{
    attribute::*,
    geometry::*,
    node::CookResult,
    session::{quick_session, Session, SessionOptions},
    Result,
};

thread_local! {
    static SESSION: Lazy<Session> = Lazy::new(|| {
        let _ = env_logger::try_init();
        let opt = SessionOptions::builder().threaded(true).build();
        let session = quick_session(Some(&opt)).expect("Could not create test session");
        session
            .load_asset_file("otls/hapi_geo.hda")
            .expect("load asset");
        session
            .load_asset_file("otls/hapi_vol.hda")
            .expect("load asset");
        session
            .load_asset_file("otls/hapi_parms.hda")
            .expect("load asset");
        session
            .load_asset_file("otls/sesi/SideFX_spaceship.hda")
            .expect("load asset");
        session
    });
}

fn _create_triangle(geo: &Geometry) {
    let part = PartInfo::default()
        .with_part_type(PartType::Mesh)
        .with_face_count(1)
        .with_point_count(3)
        .with_vertex_count(3);
    geo.set_part_info(&part).expect("part_info");
    let info = AttributeInfo::default()
        .with_count(part.point_count())
        .with_tuple_size(3)
        .with_owner(AttributeOwner::Point)
        .with_storage(StorageType::Float);
    let attr_p = geo
        .add_numeric_attribute::<f32>("P", part.part_id(), info)
        .unwrap();
    attr_p
        .set(
            part.part_id(),
            &[0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0],
        )
        .unwrap();
    geo.set_vertex_list(0, [0, 1, 2]).unwrap();
    geo.set_face_counts(0, [3]).unwrap();
    let info = AttributeInfo::default()
        .with_count(part.point_count())
        .with_tuple_size(1)
        .with_owner(AttributeOwner::Point)
        .with_storage(StorageType::Int);
    let id_attr = geo
        .add_numeric_attribute::<i32>("id", part.part_id(), info)
        .unwrap();
    id_attr.set(0, &[1, 2, 3]).unwrap();

    geo.commit().expect("commit");
    geo.node.cook_blocking().unwrap();
}

fn _create_single_point_geo(session: &Session) -> Result<Geometry> {
    let geo = session.create_input_node("dummy")?;
    let part = PartInfo::default()
        .with_part_type(PartType::Mesh)
        .with_point_count(1);
    geo.set_part_info(&part)?;
    let p_info = AttributeInfo::default()
        .with_count(part.point_count())
        .with_tuple_size(3)
        .with_owner(AttributeOwner::Point)
        .with_storage(StorageType::Float);
    let id_attr = geo.add_numeric_attribute::<f32>("P", part.part_id(), p_info)?;
    id_attr.set(part.part_id(), &[0.0, 0.0, 0.0])?;
    geo.commit().unwrap();
    geo.node.cook_blocking().unwrap();
    Ok(geo)
}

fn _load_test_geometry(session: &Session) -> Result<Geometry> {
    let node = session.create_node("Object/hapi_geo")?;
    let cook_result = node.cook_blocking().unwrap();
    assert_eq!(cook_result, CookResult::Succeeded);
    node.geometry()
        .map(|some| some.expect("must have geometry"))
}

#[test]
fn geometry_wrong_attribute() {
    SESSION.with(|session| {
        let geo = _load_test_geometry(&session).unwrap();
        let foo_bar = geo
            .get_attribute(0, AttributeOwner::Prim, "foo_bar")
            .expect("attribute");
        assert!(foo_bar.is_none());
    })
}

#[test]
fn geometry_attribute_names() {
    SESSION.with(|session| {
        let node = session.create_node("Object/hapi_geo").unwrap();
        node.cook_blocking().unwrap();
        let geo = node.geometry().unwrap().expect("geometry");
        let iter = geo
            .get_attribute_names(AttributeOwner::Point, None)
            .unwrap();
        let names: Vec<_> = iter.iter_str().collect();
        assert!(names.contains(&"Cd"));
        assert!(names.contains(&"my_float_array"));
        assert!(names.contains(&"pscale"));
        let iter = geo.get_attribute_names(AttributeOwner::Prim, None).unwrap();
        let names: Vec<_> = iter.iter_str().collect();
        assert!(names.contains(&"primname"));
        assert!(names.contains(&"shop_materialpath"));
    })
}

#[test]
fn geometry_numeric_attributes() {
    SESSION.with(|session| {
        let geo = session.create_input_node("test").unwrap();
        _create_triangle(&geo);
        // Generic way to get an attribute
        let _attr_p = geo
            .get_attribute(0, AttributeOwner::Point, "P")
            .unwrap()
            .unwrap();
        let _attr_p = _attr_p.downcast::<NumericAttr<f32>>().unwrap();
        // Convenient method
        let attr_p = geo.get_position_attribute(0).unwrap();
        let dat = attr_p.get(0).expect("read_attribute");
        assert_eq!(dat.len(), 9);
        geo.node.delete().unwrap();
    })
}

#[test]
fn geometry_create_string_attrib() {
    SESSION.with(|session| {
        let geo = session.create_input_node("test").unwrap();
        _create_triangle(&geo);
        let part = geo.part_info(0).unwrap().expect("part 0");
        let info = AttributeInfo::default()
            .with_owner(AttributeOwner::Point)
            .with_storage(StorageType::String)
            .with_tuple_size(1)
            .with_count(part.point_count());

        let attr_name = geo.add_string_attribute("name", 0, info).unwrap();
        attr_name.set(0, &["pt0", "pt1", "pt2"]).unwrap();
        geo.commit().unwrap();
        geo.node.cook_blocking().unwrap();
        let str_attr = geo
            .get_attribute(0, AttributeOwner::Point, "name")
            .unwrap()
            .unwrap();
        let Some(str_attr) = str_attr.downcast::<StringAttr>() else {
            panic!("Must be string array attr");
        };
        let str_array = str_attr.get(0).unwrap();
        let mut iter = str_array.iter_str();
        assert_eq!(iter.next(), Some("pt0"));
        assert_eq!(iter.last(), Some("pt2"));
        geo.node.delete().unwrap();
    })
}

#[test]
fn geometry_create_string_array_attrib() {
    SESSION.with(|session| {
        let geo = session.create_input_node("test").unwrap();
        _create_triangle(&geo);
        let part = geo.part_info(0).unwrap().expect("part 0");
        let info = AttributeInfo::default()
            .with_owner(AttributeOwner::Point)
            .with_storage(StorageType::StringArray)
            .with_tuple_size(1)
            .with_total_array_elements(6)
            .with_count(part.point_count());

        let array_attr = geo
            .add_string_array_attribute("my_array_a", 0, info)
            .unwrap();
        let attr_data = &["one", "two", "three", "four", "five", "six"];
        array_attr.set(attr_data.as_slice(), &[1, 2, 3]).unwrap();
        geo.commit().unwrap();
        geo.node.cook_blocking().unwrap();
        let (data, sizes) = array_attr.get(0).unwrap().flatten().unwrap();
        assert_eq!(sizes.len(), 3);
        assert_eq!(data.len(), 6);

        assert_eq!(&data[0..sizes[0]], &attr_data[0..sizes[0]]);
        assert_eq!(&data[sizes[0]..sizes[1]], &attr_data[sizes[0]..sizes[1]]);
        assert_eq!(&data[sizes[1]..sizes[2]], &attr_data[sizes[1]..sizes[2]]);
        geo.node.delete().unwrap();
    })
}

#[test]
fn geometry_read_array_attributes() {
    SESSION.with(|session| {
        let geo = _load_test_geometry(session).expect("geometry");

        let attr = geo
            .get_attribute(0, AttributeOwner::Point, "my_int_array")
            .expect("attribute")
            .unwrap();
        let attr = attr.downcast::<NumericArrayAttr<i32>>().unwrap();
        let i_array = attr.get(0).unwrap();
        assert_eq!(i_array.iter().count(), attr.info().count() as usize);
        assert_eq!(i_array.iter().next().unwrap(), &[0, 0, 0, -1]);
        assert_eq!(i_array.iter().last().unwrap(), &[7, 14, 21, -1]);

        let attr = geo
            .get_attribute(0, AttributeOwner::Point, "my_float_array")
            .expect("attribute")
            .unwrap();
        let i_array = attr.downcast::<NumericArrayAttr<f32>>().unwrap();
        let data = i_array.get(0).unwrap();

        assert_eq!(data.iter().count(), attr.info().count() as usize);
        assert_eq!(data.iter().next().unwrap(), &[0.0, 0.0, 0.0]);
        assert_eq!(data.iter().last().unwrap(), &[7.0, 14.0, 21.0]);
    });
}

#[test]
fn geometry_create_and_set_array_attributes() {
    SESSION.with(|session| {
        let input = session.create_input_node("test").unwrap();
        let part = PartInfo::default()
            .with_part_type(PartType::Mesh)
            .with_face_count(0)
            .with_vertex_count(0)
            .with_point_count(2);
        input.set_part_info(&part).unwrap();

        let p_info = AttributeInfo::default()
            .with_count(2)
            .with_tuple_size(3)
            .with_storage(StorageType::Float)
            .with_owner(AttributeOwner::Point);
        let p_attrib = input.add_numeric_attribute::<f32>("P", 0, p_info).unwrap();

        p_attrib.set(0, &[-1.0, 0.0, 0.0, 1.0, 0.0, 0.0]).unwrap();

        let data_arr = [1, 2, 3, 4, 5];
        let attr_info = AttributeInfo::default()
            .with_owner(AttributeOwner::Point)
            .with_storage(StorageType::IntArray)
            .with_total_array_elements(data_arr.len() as i64) // == to # values in DataArray
            .with_count(2) // point count
            .with_tuple_size(1);
        let array_attr = input
            .add_numeric_array_attribute::<i32>("int_array", 0, attr_info)
            .expect("attribute");
        array_attr
            .set(0, &DataArray::new(&data_arr, &[2, 3]))
            .unwrap();
        input.commit().expect("new geometry");
        input.node.cook_blocking().unwrap();
        let value = array_attr.get(0).expect("array attribute");
        assert_eq!(value.data(), &data_arr);
    })
}

#[test]
fn geometry_attribute_storage_type() -> Result<()> {
    SESSION.with(|session| {
        let geo = _load_test_geometry(session).expect("geometry");
        let attrib_list = [
            ("Cd", AttributeOwner::Point, StorageType::Float),
            ("pscale", AttributeOwner::Point, StorageType::Float),
            ("P", AttributeOwner::Point, StorageType::Float),
            ("my_int_array", AttributeOwner::Point, StorageType::IntArray),
            (
                "my_float_array",
                AttributeOwner::Point,
                StorageType::FloatArray,
            ),
            (
                "my_str_array",
                AttributeOwner::Point,
                StorageType::StringArray,
            ),
        ];
        for (name, owner, expected_storage) in attrib_list {
            let info = geo.get_attribute_info(0, owner, name)?;
            let storage = info.storage();
            assert_eq!(
                expected_storage, storage,
                "Attribute {} unexpected storage {:?} != {:?}",
                name, storage, expected_storage
            );
        }
        Ok(())
    })
}

#[test]
fn geometry_string_array_attribute() {
    SESSION.with(|session| {
        let geo = _load_test_geometry(session).expect("geometry");
        let attr = geo
            .get_attribute(0, AttributeOwner::Point, "my_str_array")
            .expect("my_str_array Point attribute")
            .unwrap();
        let attr = attr.downcast::<StringArrayAttr>().unwrap();
        let m_array = attr.get(0).unwrap();
        assert_eq!(m_array.iter().count(), attr.info().count() as usize);

        let it = m_array.iter().next().unwrap().unwrap();
        let pt_0: Vec<&str> = it.iter_str().collect();
        assert_eq!(pt_0, ["pt_0_0", "pt_0_1", "pt_0_2", "start"]);

        let it = m_array.iter().nth(1).unwrap().unwrap();
        let pt_1: Vec<&str> = it.iter_str().collect();
        assert_eq!(pt_1, ["pt_1_0", "pt_1_1", "pt_1_2"]);

        let it = m_array.iter().last().unwrap().unwrap();
        let pt_n: Vec<&str> = it.iter_str().collect();
        assert_eq!(pt_n, ["pt_7_0", "pt_7_1", "pt_7_2", "end"]);
    })
}

#[test]
fn geometry_save_and_load_to_file() {
    SESSION.with(|session| {
        let geo = session.create_input_node("triangle").unwrap();
        _create_triangle(&geo);
        let tmp_file = std::env::temp_dir().join("triangle.geo");
        geo.save_to_file(&tmp_file.to_string_lossy())
            .expect("save_to_file");
        geo.node.delete().unwrap();

        let geo = session.create_input_node("dummy").unwrap();
        geo.load_from_file(&tmp_file.to_string_lossy())
            .expect("load_from_file");
        geo.node.cook().unwrap();
        assert_eq!(geo.part_info(0).unwrap().expect("part 0").point_count(), 3);
        geo.node.delete().unwrap();
    })
}

#[test]
fn geometry_save_and_load_to_memory() {
    SESSION.with(|session| {
        let src_geo = session.create_input_node("source").unwrap();
        _create_triangle(&src_geo);
        let blob = src_geo
            .save_to_memory(GeoFormat::Geo)
            .expect("save_geo_to_memory");
        src_geo.node.delete().unwrap();

        let dest_geo = session.create_input_node("dest").unwrap();
        _create_triangle(&dest_geo);
        dest_geo
            .load_from_memory(&blob, GeoFormat::Geo)
            .expect("load_from_memory");
        dest_geo.node.delete().unwrap();
    })
}

#[test]
fn geometry_commit_and_revert() {
    SESSION.with(|session| {
        let geo = session.create_input_node("input").unwrap();
        _create_triangle(&geo);
        geo.commit().unwrap();
        geo.node.cook_blocking().unwrap();
        assert_eq!(geo.part_info(0).unwrap().expect("part 0").point_count(), 3);
        geo.revert().unwrap();
        geo.node.cook_blocking().unwrap();
        assert_eq!(geo.part_info(0).unwrap().expect("part 0").point_count(), 0);
        geo.node.delete().unwrap();
    })
}

#[test]
fn geometry_elements() {
    SESSION.with(|session| {
        let node = session.create_node("Object/hapi_geo").unwrap();
        node.cook_blocking().unwrap();
        let geo = node.geometry().unwrap().expect("Geometry");
        let part = geo.part_info(0).unwrap().expect("part 0");
        // Cube
        let points = geo
            .get_element_count_by_owner(Some(&part), AttributeOwner::Point)
            .unwrap();
        assert_eq!(points, 8);
        assert_eq!(points, part.point_count());
        let prims = geo
            .get_element_count_by_owner(Some(&part), AttributeOwner::Prim)
            .unwrap();
        assert_eq!(prims, 6);
        assert_eq!(prims, part.face_count());
        let vtx = geo
            .get_element_count_by_owner(Some(&part), AttributeOwner::Vertex)
            .unwrap();
        assert_eq!(vtx, 24);
        assert_eq!(vtx, part.vertex_count());
        let num_pt = geo
            .get_attribute_count_by_owner(Some(&part), AttributeOwner::Point)
            .unwrap();
        assert_eq!(num_pt, 8);
        let num_pr = geo
            .get_attribute_count_by_owner(Some(&part), AttributeOwner::Prim)
            .unwrap();
        assert_eq!(num_pr, 3);
        let num_det = geo
            .get_attribute_count_by_owner(Some(&part), AttributeOwner::Detail)
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
        }
    })
}

#[test]
fn geometry_delete_attribute() {
    SESSION.with(|session| {
        let geo = session.create_input_node("input").unwrap();
        _create_triangle(&geo);
        let id_attr = geo
            .get_attribute(0, AttributeOwner::Point, "id")
            .unwrap()
            .unwrap();
        id_attr.delete(0).unwrap();
        geo.commit().unwrap();
        geo.node.cook_blocking().unwrap();
        assert!(geo
            .get_attribute(0, AttributeOwner::Point, "id")
            .unwrap()
            .is_none());
    })
}

#[test]
fn geometry_partitions() {
    SESSION.with(|session| {
        let geo = session.create_input_node("input").unwrap();
        _create_triangle(&geo);
        assert_eq!(geo.partitions().unwrap().len(), 1);
        assert!(matches!(geo.part_info(100), Ok(None)));
    })
}

#[test]
fn geometry_add_and_delete_group() {
    SESSION.with(|session| {
        let geo = session.create_input_node("input").unwrap();
        _create_triangle(&geo);
        geo.add_group(0, GroupType::Point, "test", Some(&[1, 1, 1]))
            .unwrap();
        geo.commit().unwrap();
        geo.node.cook_blocking().unwrap();
        assert_eq!(
            geo.group_count_by_type(GroupType::Point, geo.geo_info().as_ref().ok())
                .unwrap(),
            1
        );

        geo.delete_group(0, GroupType::Point, "test").unwrap();
        geo.commit().unwrap();
        geo.node.cook_blocking().unwrap();
        assert_eq!(geo.group_count_by_type(GroupType::Point, None).unwrap(), 0);
        geo.node.delete().unwrap();
    })
}

#[test]
fn geometry_basic_instancing() {
    SESSION.with(|session| {
        let node = session.create_node("Object/hapi_geo").unwrap();
        node.cook_blocking().unwrap();
        let opt =
            CookOptions::default().with_packed_prim_instancing_mode(PackedPrimInstancingMode::Flat);
        node.cook_with_options(&opt, true).unwrap();
        let outputs = node.geometry_output_nodes().unwrap();
        let instancer = outputs.get(1).unwrap();
        let ids = instancer.get_instanced_part_ids(None).unwrap();
        assert_eq!(ids.len(), 1);
        let names = instancer
            .get_instance_part_groups_names(GroupType::Prim, ids[0])
            .unwrap();
        let names: Vec<String> = names.into_iter().collect();
        assert_eq!(names.first().unwrap(), "group_1");
        assert_eq!(names.last().unwrap(), "group_6");
        let transforms = instancer
            .get_instance_part_transforms(None, RSTOrder::Srt)
            .unwrap();
        assert_eq!(
            transforms.len() as i32,
            instancer
                .part_info(0)
                .unwrap()
                .expect("part 0")
                .instance_count()
        );
    })
}

#[test]
fn geometry_get_face_materials() {
    SESSION.with(|session| {
        let node = session.create_node("Object/spaceship").unwrap();
        node.cook_blocking().unwrap();
        let geo = node.geometry().expect("geometry").unwrap();
        let mats = geo.get_materials(None).expect("materials");
        assert!(matches!(mats, Some(Materials::Single(_))));
    })
}

#[test]
fn geometry_create_input_curve() {
    SESSION.with(|session| {
        let geo = session.create_input_curve_node("InputCurve").unwrap();
        let positions = &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        geo.set_input_curve_positions(0, positions).unwrap();
        let p = geo.get_position_attribute(0).unwrap();
        let coords = p.get(0).unwrap();
        assert_eq!(positions, coords.as_slice());
    })
}

#[test]
fn geometry_multiple_input_curves() {
    SESSION.with(|session| {
        let geo = session.create_input_node("InputCurves").unwrap();
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
        geo.save_to_file("c:/Temp/curve.geo").unwrap();
    })
}

#[test]
fn geometry_read_write_volume() {
    SESSION.with(|session| {
        let node = session.create_node("Object/hapi_vol").unwrap();
        node.cook_blocking().unwrap();
        let source = node.geometry().unwrap().unwrap();
        let source_part = source.part_info(0).unwrap().expect("part 0");
        let vol_info = source.volume_info(0).unwrap();
        let dest_geo = session.create_input_node("volume_copy").unwrap();
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
        dest_geo.node.cook_blocking().unwrap();
    })
}

#[test]
fn geometry_test_get_dictionary_attributes() {
    use std::collections::HashMap;
    use tinyjson::JsonValue;

    SESSION.with(|session| {
        let geo = _load_test_geometry(&session).unwrap();
        let dict_attr = geo
            .get_attribute(0, AttributeOwner::Detail, "my_dict_attr")
            .unwrap()
            .expect("my_dict_attr found");

        let dict_attr = dict_attr
            .downcast::<DictionaryAttr>()
            .expect("Attribute downcasted to DictionaryAttr");
        let values: Vec<_> = dict_attr.get(0).unwrap().into_iter().collect();
        let json_str = &values[0];
        let parsed: JsonValue = json_str.parse().expect("Could not parse attrib value json");
        let map: &HashMap<_, _> = parsed.get().expect("HashMap");
        assert_eq!(map["str_key"], JsonValue::String(String::from("text")));
        assert_eq!(map["int_key"], JsonValue::Number(1.0));
        assert!(matches!(map["list"], JsonValue::Array(_)));
        assert!(matches!(map["dict"], JsonValue::Object(_)));
    })
}

#[test]
fn geometry_test_set_dictionary_attributes() {
    use std::collections::HashMap;
    use tinyjson::JsonValue;

    SESSION.with(|session| {
        let geo = _create_single_point_geo(session).expect("Sphere geometry");
        let info = AttributeInfo::default()
            .with_count(1)
            .with_tuple_size(1)
            .with_owner(AttributeOwner::Detail)
            .with_storage(StorageType::Dictionary);
        let attr = geo
            .add_dictionary_attribute("my_dict_attr", 0, info)
            .expect("Dictionary attribute");
        let data: HashMap<String, JsonValue> = [
            ("number".to_string(), JsonValue::Number(1.0)),
            ("string".to_string(), JsonValue::String("foo".to_string())),
        ]
        .into();

        let data_str = tinyjson::stringify(&JsonValue::from(data)).expect("Json value");

        attr.set(0, &[&data_str]).unwrap();
        geo.commit().unwrap();
        geo.node.cook_blocking().unwrap();
    })
}

#[test]
fn geometry_test_get_set_dictionary_array_attribute() {
    use std::collections::HashMap;
    use tinyjson::JsonValue;

    SESSION.with(|session| {
        let geo = _create_single_point_geo(session).expect("Sphere geometry");
        let info = AttributeInfo::default()
            .with_count(2)
            .with_tuple_size(1)
            .with_owner(AttributeOwner::Detail)
            .with_storage(StorageType::DictionaryArray);
        let attr = geo
            .add_dictionary_array_attribute("my_dict_attr", 0, info)
            .expect("Dictionary array attribute");
        geo.commit().unwrap();
        geo.node.cook_blocking().unwrap();

        let dict_1: HashMap<String, JsonValue> = [
            ("number".to_string(), JsonValue::Number(1.0)),
            ("string".to_string(), JsonValue::String("foo".to_string())),
        ]
        .into();
        let dict_2: HashMap<String, JsonValue> = [
            ("number".to_string(), JsonValue::Number(3.0)),
            ("string".to_string(), JsonValue::String("bar".to_string())),
        ]
        .into();

        let data = vec![
            tinyjson::stringify(&JsonValue::from(dict_1)).expect("Json value"),
            tinyjson::stringify(&JsonValue::from(dict_2)).expect("Json value"),
        ];
        attr.set(&data, &[2])
            .expect("Dictionary array attribute set");
    })
}
