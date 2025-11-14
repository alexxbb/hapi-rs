#![cfg(feature = "async-cooking")]

use hapi_rs::attribute::{
    AsAttribute, AttributeInfo, DictionaryArrayAttr, NumericAttr, StorageType, StringArrayAttr,
    StringAttr,
};
use hapi_rs::enums::{AttributeOwner, JobStatus};
use std::collections::HashMap;
use tinyjson::JsonValue;

mod utils;

use utils::{HdaFile, create_single_point_geo, with_async_session};

#[test]
fn geometry_set_dictionary_attribute_async() {
    with_async_session(|session| {
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
        geo.commit()?;
        Ok(())
    })
    .unwrap()
}

#[test]
fn geometry_test_get_numeric_attribute_async() {
    with_async_session(|session| {
        session.load_asset_file(HdaFile::Geometry.path())?;
        let node = session.create_node("Object/hapi_geo")?;
        node.cook_blocking()?;
        let geo = node.geometry()?.expect("must have geometry");

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
    .unwrap()
}

#[test]
fn geometry_test_get_string_attribute_async() {
    with_async_session(|session| {
        session.load_asset_file(HdaFile::Geometry.path())?;
        let node = session.create_node("Object/hapi_geo")?;
        node.cook_blocking()?;
        let geo = node.geometry()?.expect("must have geometry");

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
    .unwrap()
}

#[test]
fn geometry_test_get_string_array_attribute_async() {
    with_async_session(|session| {
        session.load_asset_file(HdaFile::Geometry.path())?;
        let node = session.create_node("Object/hapi_geo")?;
        node.cook_blocking()?;
        let geo = node.geometry()?.expect("must have geometry");

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
    .unwrap()
}

#[test]
fn geometry_test_get_dictionary_array_attribute_async() {
    with_async_session(|session| {
        session.load_asset_file(HdaFile::Geometry.path())?;
        let node = session.create_node("Object/hapi_geo")?;
        node.cook_blocking()?;
        let geo = node.geometry()?.expect("must have geometry");

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
    .unwrap()
}
