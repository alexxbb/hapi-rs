use hapi_rs::attribute::{
    AsAttribute, AttributeInfo, DataArray, NumericArrayAttr, NumericAttr, StorageType,
    StringArrayAttr, StringAttr,
};
use hapi_rs::enums::{AttributeOwner, JobStatus, PartType};
use hapi_rs::geometry::{AttributeName, PartInfo};
use hapi_rs::session::{SessionInfo, SessionOptions, quick_session};
use std::ffi::CString;

mod utils;

use utils::{SessionTestExt, create_single_point_geo, create_triangle, with_session};

#[test]
fn geometry_wrong_attribute() {
    with_session(|session| {
        session.with_test_geometry(|geometry| {
            let foo_bar = geometry
                .get_attribute(0, AttributeOwner::Prim, c"foo_bar")
                .expect("attribute");
            assert!(foo_bar.is_none());
            Ok(())
        })
    })
    .unwrap()
}

#[test]
fn geometry_attribute_names() {
    with_session(|session| {
        session.with_test_geometry(|geo| {
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
    })
    .unwrap()
}

#[test]
fn geometry_numeric_attributes() {
    with_session(|session| {
        let geo = create_triangle(&session)?;
        let _attr_p = geo
            .get_attribute(0, AttributeOwner::Point, AttributeName::P)
            .unwrap()
            .unwrap();
        let _attr_p = _attr_p.downcast::<NumericAttr<f32>>().unwrap();
        let attr_p = geo.get_position_attribute(0).unwrap();
        let dat = attr_p.get(0).expect("read_attribute");
        assert_eq!(dat.len(), 9);
        geo.node.delete()
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
        let (data, sizes) = array_attr.get(0).unwrap().flatten().unwrap();
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
    with_session(|session| {
        session.with_test_geometry(|geo| {
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
    })
}

#[test]
fn geometry_string_array_attribute() {
    with_session(|session| {
        session.with_test_geometry(|geo| {
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
    })
    .unwrap()
}

#[test]
fn geometry_test_get_dictionary_attributes() {
    use hapi_rs::attribute::DictionaryAttr;
    use std::collections::HashMap;
    use tinyjson::JsonValue;

    with_session(|session| {
        session.with_test_geometry(|geo| {
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
    })
    .unwrap()
}

#[test]
#[ignore = "This test is flaky and sometimes segfaults. This seems to be fixed in Houdini 21"]
fn geometry_set_dictionary_attribute_async() -> hapi_rs::Result<()> {
    let _ = env_logger::try_init();
    let mut session_info = SessionInfo::default();
    session_info.set_connection_count(2);
    let session_options = SessionOptions::builder().session_info(session_info).build();
    let session = quick_session(Some(&session_options)).unwrap();
    let geo = create_single_point_geo(&session)?;
    let part = geo.part_info(0)?;
    let info = AttributeInfo::default()
        .with_owner(AttributeOwner::Point)
        .with_storage(StorageType::Dictionary)
        .with_tuple_size(1)
        .with_count(part.point_count());
    let attr = geo.add_dictionary_attribute("dict_attr", part.part_id(), info)?;
    let data = cr#"
    {
        "number": 1,
        "list": [1, 2, 3],
    }"#;
    let dict_array = std::iter::repeat(data)
        .take(part.point_count() as usize)
        .collect::<Vec<_>>();
    let job = attr.set_async(part.part_id(), &dict_array)?;
    while let JobStatus::Running = session.get_job_status(job)? {}
    geo.commit()
}

#[test]
fn geometry_test_get_numeric_attribute_async() {
    with_session(|session| {
        session.with_test_geometry(|geo| {
            let session = &geo.node.session;
            let float_attr = geo
                .get_attribute(0, AttributeOwner::Point, c"pscale")?
                .expect("pscale attribute");
            let attr = float_attr
                .downcast::<NumericAttr<f32>>()
                .expect("Numeric attribute");

            let part = geo.part_info(0)?;

            let mut buf = Vec::new();
            let job = attr.read_async_into(part.part_id(), &mut buf)?;
            while JobStatus::Running == session.get_job_status(job)? {}
            assert!(buf.iter().sum::<f32>() > 0.0);

            let result = attr.get_async(0)?;
            assert!(!result.is_ready()?);
            let data = result.wait()?;
            assert!(data.iter().sum::<f32>() > 0.0);
            Ok(())
        })
    })
    .unwrap()
}

#[test]
fn geometry_test_get_string_attribute_async() {
    with_session(|session| {
        session.with_test_geometry(|geo| {
            let session = &geo.node.session;
            let str_attr = geo
                .get_attribute(0, AttributeOwner::Point, c"ptname")
                .unwrap()
                .unwrap();
            let Some(attr) = str_attr.downcast::<StringAttr>() else {
                panic!("Not a string attribute");
            };

            let result = attr.get_async(0).unwrap();
            let handles = result.wait().unwrap();
            let data = session.get_string_batch(&handles).unwrap();
            assert_eq!(data.iter_str().count(), attr.info().count() as usize);
            Ok(())
        })
    })
    .unwrap()
}

#[test]
fn geometry_test_get_string_array_attribute_async() {
    with_session(|session| {
        session.with_test_geometry(|geo| {
            let session = &geo.node.session;
            let str_attr = geo
                .get_attribute(0, AttributeOwner::Point, c"my_str_array")
                .unwrap()
                .unwrap();
            let Some(attr) = str_attr.downcast::<StringArrayAttr>() else {
                panic!("Not a StringArrayAttr attribute");
            };

            let (job_id, result) = attr.get_async(0).unwrap();
            while JobStatus::Running == session.get_job_status(job_id).unwrap() {}
            let (data, sizes) = result.flatten().unwrap();
            assert_eq!(sizes[0], 4);
            let first = &data[0..sizes[0]];
            assert_eq!(&first[0], "pt_0_0");
            Ok(())
        })
    })
    .unwrap()
}

#[test]
fn geometry_test_get_dictionary_array_attribute_async() {
    use hapi_rs::attribute::DictionaryArrayAttr;
    use std::collections::HashMap;
    use tinyjson::JsonValue;

    with_session(|session| {
        session.with_test_geometry(|geo| {
            let session = &geo.node.session;
            let str_attr = geo
                .get_attribute(0, AttributeOwner::Point, c"my_dict_array_attr")
                .unwrap()
                .unwrap();
            let Some(attr) = str_attr.downcast::<DictionaryArrayAttr>() else {
                panic!("Not a DictionaryArrayAttr attribute");
            };

            let (job_id, result) = attr.get_async(0).unwrap();
            while JobStatus::Running == session.get_job_status(job_id).unwrap() {}

            let (data, sizes) = result.flatten().unwrap();
            assert_eq!(sizes[0], 0); // first point has an empty array
            let second_point = &data[sizes[0]..sizes[1]];
            assert_eq!(sizes[1], 1); // second point has one element
            let parsed: JsonValue = second_point[0]
                .parse()
                .expect("Could not parse attrib value json");
            let map: &HashMap<_, _> = parsed.get().expect("HashMap");
            assert_eq!(map["sample"], JsonValue::Number(0.0));
            Ok(())
        })
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
fn geometry_test_get_set_dictionary_array_attribute() {
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
fn test_attribute_send() {
    with_session(|session| {
        session.with_test_geometry(|geo| {
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
    })
    .unwrap()
}

#[test]
fn geometry_read_array_attributes() {
    let session = quick_session(None).unwrap();
    session
        .with_test_geometry(|geo| {
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
