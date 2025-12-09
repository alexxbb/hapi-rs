use hapi_rs::attribute::{
    AsAttribute, AttributeInfo, DataArray, DictionaryArrayAttr, NumericArrayAttr, NumericAttr,
    StorageType, StringArrayAttr, StringAttr,
};
use hapi_rs::enums::{AttributeOwner, PartType};
use hapi_rs::geometry::{AttributeName, PartInfo};
use hapi_rs::stringhandle::StringArray;
use std::ffi::CString;

mod utils;

use utils::{
    HdaFile, create_single_point_geo, create_triangle, with_session, with_session_asset,
    with_test_geometry,
};

#[test]
fn geometry_wrong_attribute() {
    with_test_geometry(|geometry| {
        let foo_bar = geometry
            .get_attribute(0, AttributeOwner::Prim, c"foo_bar")
            .expect("attribute");
        assert!(foo_bar.is_none());
        Ok(())
    })
    .unwrap()
}

#[test]
fn geometry_attribute_names() {
    with_test_geometry(|geo| {
        let part = geo.part_info(0).unwrap();
        let iter = geo
            .get_attribute_names(AttributeOwner::Point, &part)
            .unwrap();
        let names: Vec<_> = iter.iter_str().collect();
        assert!(names.contains(&"Cd"));
        assert!(names.contains(&"my_float_array"));
        assert!(names.contains(&"pscale"));
        let iter = geo
            .get_attribute_names(AttributeOwner::Prim, &part)
            .unwrap();
        let names: Vec<_> = iter.iter_str().collect();
        assert!(names.contains(&"primname"));
        assert!(names.contains(&"shop_materialpath"));
        Ok(())
    })
    .unwrap()
}

#[test]
fn geometry_numeric_attributes() {
    use hapi_rs::geometry::extra::GeometryExtension;
    with_session(|session| {
        let geo = create_triangle(&session)?;
        let _attr_p = geo
            .get_attribute(0, AttributeOwner::Point, AttributeName::P)
            .unwrap()
            .unwrap();
        let _attr_p = _attr_p.downcast::<NumericAttr<f32>>().unwrap();
        let attr_p = geo
            .get_position_attribute(&geo.part_info(0)?)
            .unwrap()
            .expect("position attribute");
        let dat = attr_p.get(0).expect("read_attribute");
        assert_eq!(dat.len(), 9);
        geo.node.delete()
    })
    .unwrap()
}

#[test]
fn numeric_attr_read_into_reuses_buffer() {
    with_session(|session| {
        session.load_asset_file(HdaFile::Geometry.path())?;
        let node = session.create_node("Object/hapi_geo")?;
        node.cook_blocking()?;
        let geo = node.geometry()?.expect("must have geometry");
        let part = geo.part_info(0)?;
        let attr = geo
            .get_attribute(0, AttributeOwner::Point, AttributeName::P)?
            .expect("P attribute");
        let attr = attr
            .downcast::<NumericAttr<f32>>()
            .expect("NumericAttr<f32>");

        let mut buffer = vec![f32::NAN; 1];
        attr.read_into(part.part_id(), &mut buffer)?;
        let expected = attr.get(part.part_id())?;
        assert_eq!(buffer, expected);

        buffer.truncate(0);
        buffer.resize(2, 123.0);
        attr.read_into(part.part_id(), &mut buffer)?;
        assert_eq!(buffer.len(), expected.len());
        assert_eq!(buffer, expected);
        node.delete()
    })
    .unwrap()
}

#[test]
fn geometry_create_string_attrib() {
    with_session(|session| {
        let geo = create_triangle(&session)?;
        let part = geo.part_info(0).unwrap();
        let info = AttributeInfo::default()
            .with_owner(AttributeOwner::Point)
            .with_storage(StorageType::String)
            .with_tuple_size(1)
            .with_count(part.point_count());

        let attr_name = geo.add_string_attribute("name", 0, info).unwrap();
        attr_name.set(0, &[c"pt0", c"pt1", c"pt2"]).unwrap();
        geo.commit().unwrap();
        geo.node.cook_blocking().unwrap();
        let str_attr = geo
            .get_attribute(0, AttributeOwner::Point, AttributeName::Name)
            .unwrap()
            .unwrap();
        let Some(str_attr) = str_attr.downcast::<StringAttr>() else {
            panic!("Must be string array attr");
        };
        let str_array = str_attr.get(0).unwrap();
        let mut iter = str_array.iter_str();
        assert_eq!(iter.next(), Some("pt0"));
        assert_eq!(iter.last(), Some("pt2"));

        geo.node.delete()
    })
    .unwrap()
}

#[test]
fn geometry_set_unique_str_attrib_value() {
    with_session(|session| {
        let geo = create_triangle(&session)?;
        let part = geo.part_info(0).unwrap();
        let info = AttributeInfo::default()
            .with_owner(AttributeOwner::Point)
            .with_storage(StorageType::String)
            .with_tuple_size(1)
            .with_count(part.point_count());

        let attr = geo.add_string_attribute("name", 0, info).unwrap();
        attr.set_unique(part.part_id(), c"unique").unwrap();
        geo.commit().unwrap();
        geo.node.cook_blocking().unwrap();

        let str_array = attr.get(part.part_id()).unwrap();
        let mut iter = str_array.iter_cstr();
        assert_eq!(iter.next(), Some(c"unique"));
        assert_eq!(iter.last(), Some(c"unique"));
        Ok(())
    })
    .unwrap()
}

#[test]
fn string_attr_set_indexed_updates_values() {
    with_session(|session| {
        session.load_asset_file(HdaFile::Geometry.path())?;
        let asset_node = session.create_node("Object/hapi_geo")?;
        asset_node.cook_blocking()?;
        let asset_geo = asset_node.geometry()?.expect("must have geometry");
        let point_count = asset_geo.part_info(0)?.point_count();
        asset_node.delete()?;

        let input = session.create_input_node("indexed_string_attr", None)?;
        let part = PartInfo::default()
            .with_part_type(PartType::Mesh)
            .with_point_count(point_count)
            .with_vertex_count(0)
            .with_face_count(0);
        input.set_part_info(&part)?;

        let p_info = AttributeInfo::default()
            .with_owner(AttributeOwner::Point)
            .with_storage(StorageType::Float)
            .with_tuple_size(3)
            .with_count(point_count);
        let p_attr = input.add_numeric_attribute::<f32>("P", 0, p_info)?;
        let positions = vec![0.0f32; (point_count * 3) as usize];
        p_attr.set(0, &positions)?;

        let attr_info = AttributeInfo::default()
            .with_owner(AttributeOwner::Point)
            .with_storage(StorageType::String)
            .with_tuple_size(1)
            .with_count(point_count);
        let attr = input.add_string_attribute("indexed_name", 0, attr_info)?;
        let values = [c"even", c"odd"];
        let indices: Vec<i32> = (0..point_count).map(|i| (i % 2) as i32).collect();
        attr.set_indexed(0, &values, &indices)?;

        input.commit()?;
        input.node.cook_blocking()?;

        let fetched = input
            .get_attribute(0, AttributeOwner::Point, c"indexed_name")?
            .expect("indexed_name attribute");
        let fetched = fetched.downcast::<StringAttr>().unwrap();
        for (idx, value) in fetched.get(0)?.iter_str().enumerate() {
            let expected = if idx % 2 == 0 { "even" } else { "odd" };
            assert_eq!(value, expected);
        }
        input.node.delete()
    })
    .unwrap()
}

#[test]
fn geometry_set_unique_int_attrib_value() {
    with_session(|session| {
        let geo = create_triangle(&session)?;
        let part = geo.part_info(0).unwrap();
        let info = AttributeInfo::default()
            .with_owner(AttributeOwner::Point)
            .with_storage(StorageType::Int)
            .with_tuple_size(2)
            .with_count(part.point_count());

        let data_size = (info.tuple_size() * info.count()) as usize;
        let attr = geo.add_numeric_attribute::<i32>("value", 0, info).unwrap();
        attr.set_unique(part.part_id(), &[8, 1]).unwrap();
        geo.commit().unwrap();
        geo.node.cook_blocking().unwrap();

        let str_array = attr.get(part.part_id()).unwrap();
        assert_eq!(str_array.len(), data_size);
        assert_eq!(&str_array[0..=1], &[8, 1]);
        Ok(())
    })
    .unwrap()
}

#[test]
fn geometry_create_string_array_attrib() {
    with_session(|session| {
        let geo = create_triangle(&session)?;
        let part = geo.part_info(0).unwrap();
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
        let attr_data_c = attr_data
            .iter()
            .map(|v| CString::new(*v).unwrap())
            .collect::<Vec<_>>();
        array_attr
            .set(0, attr_data_c.as_slice(), &[1, 2, 3])
            .unwrap();
        // NOTE: ALWAYS remember to commit AND cook after creating and setting attributes.
        geo.commit().unwrap();
        geo.node.cook_blocking().unwrap();
        let multi_array = array_attr.get(0).unwrap();
        let mut array_iter = multi_array.iter();
        assert_eq!(
            array_iter.next().unwrap().unwrap(),
            StringArray(b"one\0".to_vec())
        );
        assert_eq!(
            array_iter.next().unwrap().unwrap(),
            StringArray(b"two\0three\0".to_vec())
        );
        assert_eq!(
            array_iter.next().unwrap().unwrap(),
            StringArray(b"four\0five\0six\0".to_vec())
        );
        assert!(array_iter.next().is_none());

        let (data, sizes) = multi_array.flatten().unwrap();
        assert_eq!(sizes.len(), 3);
        assert_eq!(data.len(), 6);

        assert_eq!(&data[0..sizes[0]], &attr_data[0..sizes[0]]);
        assert_eq!(&data[sizes[0]..sizes[1]], &attr_data[sizes[0]..sizes[1]]);
        assert_eq!(&data[sizes[1]..sizes[2]], &attr_data[sizes[1]..sizes[2]]);
        geo.node.delete()
    })
    .unwrap()
}

#[test]
fn geometry_attribute_storage_type() -> hapi_rs::Result<()> {
    with_test_geometry(|geo| {
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
    with_test_geometry(|geo| {
        let attr = geo
            .get_attribute(0, AttributeOwner::Point, c"my_str_array")
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
        Ok(())
    })
    .unwrap()
}

#[test]
fn geometry_test_get_dictionary_attributes() {
    use hapi_rs::attribute::DictionaryAttr;
    use std::collections::HashMap;
    use tinyjson::JsonValue;

    with_test_geometry(|geo| {
        let dict_attr = geo
            .get_attribute(0, AttributeOwner::Detail, c"my_dict_attr")
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
        Ok(())
    })
    .unwrap()
}

#[test]
fn dictionary_array_attr_get_returns_expected_values() {
    use std::collections::HashMap;
    use std::str::FromStr;
    use tinyjson::JsonValue;

    with_session_asset(HdaFile::Geometry, |lib| {
        let asset = lib.try_create_first().expect("create_node");
        let geo = asset.geometry()?.expect("must have geometry");
        geo.node.cook_blocking().expect("cook_blocking");

        let dict_array = geo
            .get_attribute(0, AttributeOwner::Point, c"my_dict_array_attr")?
            .expect("dictionary array attribute");
        let dict_array = dict_array
            .downcast::<DictionaryArrayAttr>()
            .expect("DictionaryArrayAttr");
        let arrays = dict_array.get(0)?;
        assert_eq!(arrays.iter().count(), dict_array.info().count() as usize);
        let (flat, sizes) = arrays.flatten().unwrap();
        assert_eq!(sizes.len(), dict_array.info().count() as usize);
        assert_eq!(flat.len(), sizes.iter().sum::<usize>());
        assert_eq!(sizes.first().copied().unwrap_or_default(), 0);

        let mut start = 0;
        for (index, size) in sizes.iter().enumerate() {
            if index == 1 && *size > 0 {
                let slice = &flat[start..start + *size];
                let parsed =
                    JsonValue::from_str(&slice[0]).expect("Json value from dictionary array");
                let map: &HashMap<_, _> = parsed.get().expect("HashMap");
                assert_eq!(map["sample"], JsonValue::Number(0.0));
                break;
            }
            start += *size;
        }
        Ok(())
    })
    .unwrap()
}

#[test]
fn geometry_test_set_dictionary_attributes() {
    use std::collections::HashMap;
    use std::str::FromStr;
    use tinyjson::JsonValue;

    with_session(|session| {
        let geo = create_single_point_geo(&session).expect("Sphere geometry");
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

        let data_str = tinyjson::stringify(&JsonValue::from(data.clone())).expect("Json value");

        attr.set(0, &[&CString::new(data_str).unwrap()]).unwrap();
        geo.commit().unwrap();
        geo.node.cook_blocking().unwrap();
        let value: Vec<_> = attr.get(0).unwrap().into_iter().collect();
        let new_data =
            JsonValue::from_str(&value[0]).expect("Json value from string attriubute value");

        assert_eq!(JsonValue::from(data), new_data);
        Ok(())
    })
    .unwrap()
}

#[test]
fn geometry_get_set_dictionary_array_attribute() {
    use std::collections::HashMap;
    use tinyjson::JsonValue;

    with_session(|session| {
        let geo = create_single_point_geo(&session).expect("Sphere geometry");
        let info = AttributeInfo::default()
            .with_count(1)
            .with_tuple_size(1)
            .with_owner(AttributeOwner::Detail)
            .with_storage(StorageType::DictionaryArray);
        let attr = geo
            .add_dictionary_array_attribute("my_dict_attr", 0, info)
            .expect("Dictionary array attribute");

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
            CString::new(tinyjson::stringify(&JsonValue::from(dict_1)).expect("Json value"))
                .unwrap(),
            CString::new(tinyjson::stringify(&JsonValue::from(dict_2)).expect("Json value"))
                .unwrap(),
        ];
        attr.set(0, &data, &[2])
            .expect("Dictionary array attribute set");
        geo.commit()
    })
    .unwrap()
}

#[test]
fn attribute_send_to_thread() {
    with_test_geometry(|geo| {
        let str_attr = geo
            .get_attribute(0, AttributeOwner::Point, c"pscale")
            .unwrap()
            .unwrap();
        std::thread::spawn(move || {
            if let Some(attr) = str_attr.downcast::<NumericAttr<f32>>() {
                let _ = attr.get(0);
            }
        })
        .join()
        .unwrap();
        Ok(())
    })
    .unwrap()
}

#[test]
fn geometry_read_array_attributes() {
    with_test_geometry(|geo| {
        let attr = geo
            .get_attribute(0, AttributeOwner::Point, c"my_int_array")
            .expect("attribute")
            .unwrap();
        let attr = attr.downcast::<NumericArrayAttr<i32>>().unwrap();
        let i_array = attr.get(0).unwrap();
        assert_eq!(i_array.iter().count(), attr.info().count() as usize);
        assert_eq!(i_array.iter().next().unwrap(), &[0, 0, 0, -1]);
        assert_eq!(i_array.iter().last().unwrap(), &[7, 14, 21, -1]);

        let attr = geo
            .get_attribute(0, AttributeOwner::Point, c"my_float_array")
            .expect("attribute")
            .unwrap();
        let i_array = attr.downcast::<NumericArrayAttr<f32>>().unwrap();
        let data = i_array.get(0).unwrap();

        assert_eq!(data.iter().count(), attr.info().count() as usize);
        assert_eq!(data.iter().next().unwrap(), &[0.0, 0.0, 0.0]);
        assert_eq!(data.iter().last().unwrap(), &[7.0, 14.0, 21.0]);
        Ok(())
    })
    .unwrap()
}

#[test]
fn geometry_create_and_set_array_attributes() {
    with_session(|session| {
        let input = session.create_input_node("test", None).unwrap();
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
        Ok(())
    })
    .unwrap()
}
