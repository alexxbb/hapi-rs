use hapi_rs::Result;
use hapi_rs::attribute::{
    AnyAttribute, AttributeInfo, DictionaryAttribute, Fixed, Jagged, JaggedArrayData, StorageType,
    StringAttribute,
};
use hapi_rs::enums::{AttributeOwner, PartType};
use hapi_rs::geometry::{AttributeName, PartInfo};
use pretty_assertions::assert_eq;
use std::ffi::CString;

mod utils;
use utils::{create_single_point_geo, create_triangle, with_session, with_test_geometry};

#[test]
fn missing_and_typed_lookup() -> Result<()> {
    with_test_geometry(|geo| {
        assert!(
            geo.get_attribute(0, AttributeOwner::Point, c"missing")?
                .is_none()
        );
        assert!(
            geo.get_numeric_attribute::<f32, Fixed>(0, AttributeOwner::Point, c"P")?
                .is_some()
        );
        assert!(
            geo.get_numeric_attribute::<f32, Fixed>(0, AttributeOwner::Point, c"missing")?
                .is_none()
        );
        assert!(
            geo.get_numeric_attribute::<i32, Fixed>(0, AttributeOwner::Point, c"P")
                .is_err()
        );
        assert!(
            geo.get_string_attribute::<Fixed>(0, AttributeOwner::Point, c"P")
                .is_err()
        );
        Ok(())
    })
}

#[test]
fn fixture_maps_to_exhaustive_variants() -> Result<()> {
    with_test_geometry(|geo| {
        let cases = [
            (AttributeOwner::Point, c"P", StorageType::Float),
            (AttributeOwner::Point, c"ptname", StorageType::String),
            (
                AttributeOwner::Point,
                c"my_int_array",
                StorageType::IntArray,
            ),
            (
                AttributeOwner::Point,
                c"my_float_array",
                StorageType::FloatArray,
            ),
            (
                AttributeOwner::Point,
                c"my_str_array",
                StorageType::StringArray,
            ),
            (
                AttributeOwner::Point,
                c"my_dict_array_attr",
                StorageType::DictionaryArray,
            ),
            (
                AttributeOwner::Detail,
                c"my_dict_attr",
                StorageType::Dictionary,
            ),
        ];
        for (owner, name, storage) in cases {
            let attr = geo
                .get_attribute(0, owner, name)?
                .expect("fixture attribute");
            assert_eq!(attr.storage(), storage);
            assert_eq!(attr.part_id(), 0);
            assert_eq!(attr.owner(), owner);
            match (storage, attr) {
                (StorageType::Float, AnyAttribute::Float(_))
                | (StorageType::String, AnyAttribute::String(_))
                | (StorageType::IntArray, AnyAttribute::IntArray(_))
                | (StorageType::FloatArray, AnyAttribute::FloatArray(_))
                | (StorageType::StringArray, AnyAttribute::StringArray(_))
                | (StorageType::Dictionary, AnyAttribute::Dictionary(_))
                | (StorageType::DictionaryArray, AnyAttribute::DictionaryArray(_)) => {}
                (_, other) => panic!("wrong variant: {other:?}"),
            }
        }
        Ok(())
    })
}

#[test]
fn fixed_tuple_read_and_length_validation() -> Result<()> {
    with_session(|session| {
        let geo = create_triangle(&session)?;
        let attr = geo
            .get_numeric_attribute::<f32, Fixed>(0, AttributeOwner::Point, AttributeName::P)?
            .unwrap();
        assert_eq!(attr.get()?.len(), 9);
        assert!(attr.set(&[0.0; 8]).is_err());
        attr.set(&[0.0; 9])?;
        Ok(())
    })
}

#[test]
fn numeric_jagged_read_and_safe_iteration() -> Result<()> {
    with_test_geometry(|geo| {
        let attr = geo
            .get_numeric_attribute::<i32, Jagged>(0, AttributeOwner::Point, c"my_int_array")?
            .unwrap();
        let data = attr.get()?;
        assert_eq!(data.iter().next(), Some(&[0, 0, 0, -1][..]));
        assert_eq!(data.iter().last(), Some(&[7, 14, 21, -1][..]));
        assert_eq!(data.iter().count(), data.sizes().len());
        Ok(())
    })
}

#[test]
fn string_and_dictionary_jagged_reads() -> Result<()> {
    with_test_geometry(|geo| {
        let strings = geo
            .get_string_attribute::<Jagged>(0, AttributeOwner::Point, c"my_str_array")?
            .unwrap();
        let strings = strings.get()?;
        let first = strings.iter().next().unwrap()?;
        assert_eq!(
            first.iter_str().collect::<Vec<_>>(),
            ["pt_0_0", "pt_0_1", "pt_0_2", "start"]
        );

        let dictionaries = geo
            .get_dictionary_attribute::<Jagged>(0, AttributeOwner::Point, c"my_dict_array_attr")?
            .unwrap();
        let (flat, sizes) = dictionaries.get()?.flatten()?;
        assert_eq!(flat.len(), sizes.iter().sum::<usize>());
        Ok(())
    })
}

#[test]
fn creation_derives_storage_and_rejects_bad_lengths() -> Result<()> {
    with_session(|session| {
        let geo = create_single_point_geo(&session)?;
        let info = AttributeInfo::default()
            .with_count(1)
            .with_tuple_size(1)
            .with_owner(AttributeOwner::Point)
            .with_storage(StorageType::Invalid)
            .with_total_array_elements(2);
        let fixed = geo.add_numeric_attribute::<i64, Fixed>("fixed", 0, info.clone())?;
        assert_eq!(fixed.info().storage(), StorageType::Int64);
        fixed.set(&[4])?;
        let jagged = geo.add_numeric_attribute::<i64, Jagged>("jagged", 0, info)?;
        assert_eq!(jagged.info().storage(), StorageType::Int64Array);
        assert!(
            jagged
                .set(&JaggedArrayData::new(vec![1], vec![1, 0])?)
                .is_err()
        );
        jagged.set(&JaggedArrayData::new(vec![1, 2], vec![2])?)?;
        Ok(())
    })
}

#[test]
fn fixed_string_unique_indexed_and_dictionary() -> Result<()> {
    with_session(|session| {
        let input = session.create_input_node("string_attributes", None)?;
        let part = PartInfo::default()
            .with_part_type(PartType::Mesh)
            .with_point_count(2);
        input.set_part_info(&part)?;
        let info = AttributeInfo::default()
            .with_owner(AttributeOwner::Point)
            .with_storage(StorageType::String)
            .with_tuple_size(1)
            .with_count(2);
        let strings: StringAttribute<Fixed> = input.add_string_attribute("name", 0, info)?;
        strings.set_unique(c"same")?;
        strings.set_indexed(&[c"left", c"right"], &[0, 1])?;

        let info = AttributeInfo::default()
            .with_owner(AttributeOwner::Detail)
            .with_storage(StorageType::Dictionary)
            .with_tuple_size(1)
            .with_count(1);
        let dictionary: DictionaryAttribute<Fixed> =
            input.add_dictionary_attribute("data", 0, info)?;
        dictionary.set(&[c"{\"value\":1}"])?;
        Ok(())
    })
}

#[test]
fn deletion_and_send_preserve_identity() -> Result<()> {
    with_session(|session| {
        let geo = create_triangle(&session)?;
        let attr = geo
            .get_numeric_attribute::<f32, Fixed>(0, AttributeOwner::Point, c"P")?
            .unwrap();
        assert_eq!(attr.part_id(), 0);
        assert_eq!(attr.owner(), AttributeOwner::Point);
        std::thread::spawn(move || attr.get()).join().unwrap()?;

        let info = AttributeInfo::default()
            .with_count(1)
            .with_tuple_size(1)
            .with_owner(AttributeOwner::Detail);
        let temp = geo.add_numeric_attribute::<i32, Fixed>("temporary", 0, info)?;
        temp.delete()?;
        assert!(
            geo.get_attribute(0, AttributeOwner::Detail, c"temporary")?
                .is_none()
        );
        Ok(())
    })
}

#[test]
fn jagged_constructor_validation() {
    assert!(JaggedArrayData::new(vec![1], vec![-1]).is_err());
    assert!(JaggedArrayData::new(vec![1], vec![2]).is_err());
}

#[allow(dead_code)]
fn _cstring_is_supported(_: CString) {}
