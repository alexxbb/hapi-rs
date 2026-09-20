#![cfg(feature = "async-cooking")]

use hapi_rs::Result;
use hapi_rs::attribute::{
    AsyncAttributeAccess, AsyncFixedAttributeAccess, AsyncStringAttributeAccess, AttributeInfo,
    DictionaryAttribute, Fixed, Jagged, JaggedArrayData, StorageType, StringAttribute,
};
use hapi_rs::enums::AttributeOwner;
use std::ffi::CString;

mod utils;
use utils::{create_single_point_geo, with_async_session, with_async_test_geometry};

#[test]
fn async_fixed_and_jagged_gets_complete() -> Result<()> {
    with_async_test_geometry(|geo| {
        let fixed = geo
            .get_numeric_attribute::<f32, Fixed>(0, AttributeOwner::Point, c"pscale")?
            .unwrap();
        let job = fixed.get_async()?;
        assert!(job.job_id() >= 0);
        assert!(!job.wait()?.is_empty());

        let jagged = geo
            .get_numeric_attribute::<i32, Jagged>(0, AttributeOwner::Point, c"my_int_array")?
            .unwrap();
        assert!(!jagged.get_async()?.wait()?.data().is_empty());

        let strings = geo
            .get_string_attribute::<Jagged>(0, AttributeOwner::Point, c"my_str_array")?
            .unwrap();
        assert!(!strings.get_async()?.wait()?.flatten()?.0.is_empty());

        let dictionaries = geo
            .get_dictionary_attribute::<Jagged>(0, AttributeOwner::Point, c"my_dict_array_attr")?
            .unwrap();
        let (_, sizes) = dictionaries.get_async()?.wait()?.flatten()?;
        assert!(!sizes.is_empty());
        Ok(())
    })
}

#[test]
fn async_set_unique_and_indexed_operations() -> Result<()> {
    with_async_session(|session| {
        let geo = create_single_point_geo(&session)?;
        let fixed_info = AttributeInfo::default()
            .with_count(1)
            .with_tuple_size(1)
            .with_owner(AttributeOwner::Point);
        let numeric = geo.add_numeric_attribute::<i32, Fixed>("number", 0, fixed_info.clone())?;
        numeric.set_async(&[2])?.wait()?;
        numeric.set_unique_async(&[3])?.wait()?;

        let string_info = fixed_info.clone().with_storage(StorageType::String);
        let string: StringAttribute<Fixed> = geo.add_string_attribute("name", 0, string_info)?;
        string.set_async(&[CString::new("value")?])?.wait()?;
        string.set_unique_async(c"unique")?.wait()?;
        string.set_indexed_async(&[c"indexed"], &[0])?.wait()?;

        let dict_info = fixed_info.clone().with_storage(StorageType::Dictionary);
        let dictionary: DictionaryAttribute<Fixed> =
            geo.add_dictionary_attribute("dict", 0, dict_info)?;
        dictionary
            .set_async(&[CString::new("{\"value\":1}")?])?
            .wait()?;

        let jagged_info = fixed_info.with_total_array_elements(2);
        let jagged = geo.add_numeric_attribute::<i32, Jagged>("numbers", 0, jagged_info)?;
        jagged
            .set_async(&JaggedArrayData::new(vec![1, 2], vec![2])?)?
            .wait()?;
        Ok(())
    })
}

#[test]
fn early_drop_keeps_ffi_storage_alive() -> Result<()> {
    with_async_test_geometry(|geo| {
        let fixed = geo
            .get_numeric_attribute::<f32, Fixed>(0, AttributeOwner::Point, c"pscale")?
            .unwrap();
        drop(fixed.get_async()?);
        Ok(())
    })
}
