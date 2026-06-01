// FIXME: This test is disabled because async cooking is sill not stable in Houdini
#![cfg(feature = "async-cooking")]

use hapi_rs::Result;
use hapi_rs::attribute::{
    AsAttribute, AttributeInfo, DictionaryArrayAttr, NumericAttr, StorageType, StringArrayAttr,
    StringAttr,
};
use hapi_rs::enums::{AttributeOwner, JobStatus};
use pretty_assertions::assert_eq;
use std::collections::HashMap;
use tinyjson::JsonValue;

mod utils;

use utils::{HdaFile, create_single_point_geo, with_async_session};

#[test]
fn geometry_set_dictionary_attribute_async() -> Result<()> {
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
}

#[test]
fn geometry_test_get_numeric_attribute_async() -> Result<()> {
    with_async_session(|session| {
        session.load_asset_file(HdaFile::Geometry.path())?;
        let node = session.create_node("Object/hapi_geo")?;
        node.cook_blocking()?;
        let geo = node
            .geometry()?
            .ok_or_else(|| hapi_rs::HapiError::Internal("must have geometry".into()))?;

        let float_attr = geo
            .get_attribute(0, AttributeOwner::Point, c"pscale")?
            .ok_or_else(|| hapi_rs::HapiError::Internal("pscale attribute".into()))?;
        let attr = float_attr
            .downcast::<NumericAttr<f32>>()
            .ok_or_else(|| hapi_rs::HapiError::Internal("Numeric attribute".into()))?;

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
}

#[test]
fn geometry_test_get_string_attribute_async() -> Result<()> {
    with_async_session(|session| {
        session.load_asset_file(HdaFile::Geometry.path())?;
        let node = session.create_node("Object/hapi_geo")?;
        node.cook_blocking()?;
        let geo = node
            .geometry()?
            .ok_or_else(|| hapi_rs::HapiError::Internal("must have geometry".into()))?;

        let str_attr = geo
            .get_attribute(0, AttributeOwner::Point, c"ptname")?
            .ok_or_else(|| hapi_rs::HapiError::Internal("ptname attribute".into()))?;
        let Some(attr) = str_attr.downcast::<StringAttr>() else {
            return Err(hapi_rs::HapiError::Internal(
                "Not a string attribute".into(),
            ));
        };

        let result = attr.get_async(0)?;
        let handles = result.wait()?;
        let data = session.get_string_batch(&handles)?;
        assert_eq!(data.iter_str().count(), attr.info().count() as usize);
        Ok(())
    })
}

#[test]
fn geometry_test_get_string_array_attribute_async() -> Result<()> {
    with_async_session(|session| {
        session.load_asset_file(HdaFile::Geometry.path())?;
        let node = session.create_node("Object/hapi_geo")?;
        node.cook_blocking()?;
        let geo = node
            .geometry()?
            .ok_or_else(|| hapi_rs::HapiError::Internal("must have geometry".into()))?;

        let str_attr = geo
            .get_attribute(0, AttributeOwner::Point, c"my_str_array")?
            .ok_or_else(|| hapi_rs::HapiError::Internal("my_str_array attribute".into()))?;
        let Some(attr) = str_attr.downcast::<StringArrayAttr>() else {
            return Err(hapi_rs::HapiError::Internal(
                "Not a StringArrayAttr attribute".into(),
            ));
        };

        let (job_id, result) = attr.get_async(0)?;
        while JobStatus::Running == session.get_job_status(job_id)? {}
        let (data, sizes) = result.flatten()?;
        assert_eq!(sizes[0], 4);
        let first = &data[0..sizes[0]];
        assert_eq!(&first[0], "pt_0_0");
        Ok(())
    })
}

#[test]
fn geometry_test_get_dictionary_array_attribute_async() -> Result<()> {
    with_async_session(|session| {
        session.load_asset_file(HdaFile::Geometry.path())?;
        let node = session.create_node("Object/hapi_geo")?;
        node.cook_blocking()?;
        let geo = node
            .geometry()?
            .ok_or_else(|| hapi_rs::HapiError::Internal("must have geometry".into()))?;

        let str_attr = geo
            .get_attribute(0, AttributeOwner::Point, c"my_dict_array_attr")?
            .ok_or_else(|| hapi_rs::HapiError::Internal("my_dict_array_attr attribute".into()))?;
        let Some(attr) = str_attr.downcast::<DictionaryArrayAttr>() else {
            return Err(hapi_rs::HapiError::Internal(
                "Not a DictionaryArrayAttr attribute".into(),
            ));
        };

        let (job_id, result) = attr.get_async(0)?;
        while JobStatus::Running == session.get_job_status(job_id)? {}

        let (data, sizes) = result.flatten()?;
        assert_eq!(sizes[0], 0); // first point has an empty array
        let second_point = &data[sizes[0]..sizes[1]];
        assert_eq!(sizes[1], 1); // second point has one element
        let parsed: JsonValue = second_point[0].parse().map_err(|e| {
            hapi_rs::HapiError::Internal(format!("Could not parse attrib value json: {e}"))
        })?;
        let map: &HashMap<_, _> = parsed
            .get()
            .ok_or_else(|| hapi_rs::HapiError::Internal("HashMap".into()))?;
        assert_eq!(map["sample"], JsonValue::Number(0.0));
        Ok(())
    })
}
