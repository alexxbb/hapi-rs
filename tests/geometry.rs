use once_cell::sync::Lazy;

use hapi_rs::{
    attribute::*,
    geometry::*,
    session::{quick_session, Session, SessionOptions},
    Result,
};

static SESSION: Lazy<Session> = Lazy::new(|| {
    env_logger::init();
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
    geo.commit().expect("commit");
    geo.node.cook_blocking().unwrap();
}

fn _load_test_geometry(session: &Session) -> Result<Geometry> {
    let node = session.create_node("Object/hapi_geo")?;
    node.cook_blocking().unwrap();
    node.geometry()
        .map(|some| some.expect("must have geometry"))
}

#[test]
fn geometry_wrong_attribute() {
    let geo = _load_test_geometry(&SESSION).unwrap();
    let foo_bar = geo
        .get_attribute(0, AttributeOwner::Prim, "foo_bar")
        .expect("attribute");
    assert!(foo_bar.is_none());
}

#[test]
fn geometry_attribute_names() {
    let node = SESSION.create_node("Object/hapi_geo").unwrap();
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
}

#[test]
fn geometry_numeric_attributes() {
    let geo = SESSION.create_input_node("test").unwrap();
    _create_triangle(&geo);
    let attr_p = geo
        .get_attribute(0, AttributeOwner::Point, "P")
        .unwrap()
        .unwrap();
    let attr_p = attr_p.downcast::<NumericAttr<f32>>().unwrap();
    let dat = attr_p.get(0).expect("read_attribute");
    assert_eq!(dat.len(), 9);
    geo.node.delete().unwrap();
}

#[test]
fn geometry_create_string_attrib() {
    let geo = SESSION.create_input_node("test").unwrap();
    _create_triangle(&geo);
    let part = geo.part_info(0).unwrap();
    let info = AttributeInfo::default()
        .with_owner(AttributeOwner::Point)
        .with_storage(StorageType::String)
        .with_tuple_size(1)
        .with_count(part.point_count());

    let attr_name = geo.add_string_attribute("name", 0, info).unwrap();
    attr_name.set(0, &["pt0", "pt1", "pt2"]).unwrap();
    geo.commit().unwrap();
    geo.node.delete().unwrap();
}

#[test]
fn geometry_array_attributes() {
    let geo = _load_test_geometry(&SESSION).expect("geometry");

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
}

#[test]
fn geometry_string_array_attribute() {
    let geo = _load_test_geometry(&SESSION).expect("geometry");
    let attr = geo
        .get_attribute(0, AttributeOwner::Point, "my_str_array")
        .expect("attribute")
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
}

#[test]
fn geometry_save_and_load_to_file() {
    let geo = SESSION.create_input_node("triangle").unwrap();
    _create_triangle(&geo);
    let tmp_file = std::env::temp_dir().join("triangle.geo");
    geo.save_to_file(&tmp_file.to_string_lossy())
        .expect("save_to_file");
    geo.node.delete().unwrap();

    let geo = SESSION.create_input_node("dummy").unwrap();
    geo.load_from_file(&tmp_file.to_string_lossy())
        .expect("load_from_file");
    geo.node.cook().unwrap();
    assert_eq!(geo.part_info(0).unwrap().point_count(), 3);
    geo.node.delete().unwrap();
}

#[test]
fn geometry_save_and_load_to_memory() {
    let src_geo = SESSION.create_input_node("source").unwrap();
    _create_triangle(&src_geo);
    let blob = src_geo
        .save_to_memory(GeoFormat::Geo)
        .expect("save_geo_to_memory");
    src_geo.node.delete().unwrap();

    let dest_geo = SESSION.create_input_node("dest").unwrap();
    _create_triangle(&dest_geo);
    dest_geo
        .load_from_memory(&blob, GeoFormat::Geo)
        .expect("load_from_memory");
    dest_geo.node.delete().unwrap();
}

#[test]
fn geometry_commit_and_revert() {
    let geo = SESSION.create_input_node("input").unwrap();
    _create_triangle(&geo);
    geo.commit().unwrap();
    geo.node.cook_blocking().unwrap();
    assert_eq!(geo.part_info(0).unwrap().point_count(), 3);
    geo.revert().unwrap();
    geo.node.cook_blocking().unwrap();
    assert_eq!(geo.part_info(0).unwrap().point_count(), 0);
    geo.node.delete().unwrap();
}

#[test]
fn geometry_elements() {
    let node = SESSION.create_node("Object/hapi_geo").unwrap();
    node.cook_blocking().unwrap();
    let geo = node.geometry().unwrap().expect("Geometry");
    let part = geo.part_info(0).unwrap();
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
    assert_eq!(num_pt, 7);
    let num_pr = geo
        .get_attribute_count_by_owner(Some(&part), AttributeOwner::Prim)
        .unwrap();
    assert_eq!(num_pr, 3);
    let pr_groups = geo.get_group_names(GroupType::Prim).unwrap();
    let pt_groups = geo.get_group_names(GroupType::Point).unwrap();
    #[allow(clippy::needless_collect)]
    {
        let pr_groups = pr_groups.iter_str().collect::<Vec<_>>();
        let pt_groups = pt_groups.iter_str().collect::<Vec<_>>();
        assert!(pr_groups.contains(&"group_A"));
        assert!(pt_groups.contains(&"group_B"));
    }
}

#[test]
fn geometry_add_and_delete_group() {
    let geo = SESSION.create_input_node("input").unwrap();
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
}

#[test]
fn geometry_basic_instancing() {
    let node = SESSION.create_node("Object/hapi_geo").unwrap();
    node.cook_blocking().unwrap();
    let opt =
        CookOptions::default().with_packed_prim_instancing_mode(PackedPrimInstancingMode::Flat);
    node.cook_with_options(&opt, true).unwrap();
    let outputs = node.geometry_outputs().unwrap();
    let instancer = outputs.get(1).unwrap();
    let ids = instancer.get_instanced_part_ids(None).unwrap();
    assert_eq!(ids.len(), 1);
    let names = instancer
        .get_instance_part_groups_names(GroupType::Prim, ids[0])
        .unwrap();
    let names: Vec<String> = names.into_iter().collect();
    assert_eq!(names.first().unwrap(), "group_1");
    assert_eq!(names.last().unwrap(), "group_6");
    let tranforms = instancer
        .get_instance_part_transforms(None, RSTOrder::Srt)
        .unwrap();
    assert_eq!(
        tranforms.len() as i32,
        instancer.part_info(0).unwrap().instance_count()
    );
}

#[test]
fn geometry_get_face_materials() {
    let node = SESSION.create_node("Object/spaceship").unwrap();
    node.cook_blocking().unwrap();
    let geo = node.geometry().expect("geometry").unwrap();
    let mats = geo.get_materials(None).expect("materials");
    assert!(matches!(mats, Some(Materials::Single(_))));
}

#[test]
fn geometry_create_input_curve() {
    let geo = SESSION.create_input_curve_node("InputCurve").unwrap();
    let positions = &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
    geo.set_input_curve_positions(0, positions).unwrap();
    let p = geo.get_position_attribute(0).unwrap();
    let coords = p.get(0).unwrap();
    assert_eq!(positions, coords.as_slice());
}

#[test]
fn geometry_read_write_volume() {
    let node = SESSION.create_node("Object/hapi_vol").unwrap();
    node.cook_blocking().unwrap();
    let source = node.geometry().unwrap().unwrap();
    let source_part = source.part_info(0).unwrap();
    let vol_info = source.volume_info(0).unwrap();
    let dest_geo = SESSION.create_input_node("volume_copy").unwrap();
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
}
