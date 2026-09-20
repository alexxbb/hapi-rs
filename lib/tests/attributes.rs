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
        let mut position = geo
            .get_numeric_attribute::<f32, Fixed>(0, AttributeOwner::Point, c"P")?
            .unwrap();
        let type_info = position.info().type_info();
        let original_owner = position.info().original_owner();
        position.refresh()?;
        assert_eq!(position.info().storage(), StorageType::Float);
        assert_eq!(position.info().type_info(), type_info);
        assert_eq!(position.info().original_owner(), original_owner);
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
        let first = strings.iter().next().unwrap();
        assert_eq!(
            first.iter().map(String::as_str).collect::<Vec<_>>(),
            ["pt_0_0", "pt_0_1", "pt_0_2", "start"]
        );

        // The result owns its strings and remains valid after more HAPI string
        // operations on the same session.
        let _ = geo.get_attribute_names(AttributeOwner::Point, &geo.part_info(0)?)?;
        assert_eq!(strings.iter().next().unwrap()[0], "pt_0_0");

        let dictionaries = geo
            .get_dictionary_attribute::<Jagged>(0, AttributeOwner::Point, c"my_dict_array_attr")?
            .unwrap();
        let dictionaries = dictionaries.get()?;
        assert_eq!(
            dictionaries.data().len(),
            dictionaries
                .sizes()
                .iter()
                .map(|&v| usize::try_from(v).unwrap())
                .sum::<usize>()
        );
        Ok(())
    })
}

#[test]
fn fixed_numeric_ranges_are_element_based_and_checked() -> Result<()> {
    with_session(|session| {
        let geo = create_triangle(&session)?;
        let position = geo
            .get_numeric_attribute::<f32, Fixed>(0, AttributeOwner::Point, AttributeName::P)?
            .unwrap();
        assert_eq!(position.get_range(1..3)?.len(), 6);
        assert!(position.get_range(3..3)?.is_empty());
        let reversed = std::ops::Range { start: 2, end: 1 };
        assert!(position.get_range(reversed).is_err());
        assert!(position.get_range(0..4).is_err());
        assert!(position.set_range(1..3, &[0.0; 5]).is_err());
        position.set_range(1..3, &[0.0; 6])?;
        geo.commit()?;
        geo.node.cook_blocking()?;
        let position = geo
            .get_numeric_attribute::<f32, Fixed>(0, AttributeOwner::Point, AttributeName::P)?
            .unwrap();
        assert_eq!(position.get_range(1..3)?, vec![0.0; 6]);
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
        let p_info = AttributeInfo::default()
            .with_owner(AttributeOwner::Point)
            .with_storage(StorageType::Float)
            .with_tuple_size(3)
            .with_count(2);
        input
            .add_numeric_attribute::<f32, Fixed>("P", 0, p_info)?
            .set(&[0.0; 6])?;
        let info = AttributeInfo::default()
            .with_owner(AttributeOwner::Point)
            .with_storage(StorageType::String)
            .with_tuple_size(1)
            .with_count(2);
        let strings: StringAttribute<Fixed> = input.add_string_attribute("name", 0, info)?;
        strings.set_unique(c"héllo")?;
        let tuple_info = AttributeInfo::default()
            .with_owner(AttributeOwner::Point)
            .with_storage(StorageType::String)
            .with_tuple_size(2)
            .with_count(2);
        input
            .add_string_attribute("tuple_name", 0, tuple_info)?
            .set_unique(c"pair")?;

        let info = AttributeInfo::default()
            .with_owner(AttributeOwner::Detail)
            .with_storage(StorageType::Dictionary)
            .with_tuple_size(1)
            .with_count(1);
        let dictionary: DictionaryAttribute<Fixed> =
            input.add_dictionary_attribute("data", 0, info)?;
        dictionary.set(&[c"{\"value\":1}"])?;

        input.commit()?;
        input.node.cook_blocking()?;
        let strings = input
            .get_string_attribute::<Fixed>(0, AttributeOwner::Point, c"name")?
            .unwrap();
        assert_eq!(
            strings.get()?.iter_str().collect::<Vec<_>>(),
            ["héllo", "héllo"]
        );
        let tuple_strings = input
            .get_string_attribute::<Fixed>(0, AttributeOwner::Point, c"tuple_name")?
            .unwrap();
        assert_eq!(
            tuple_strings.get()?.iter_str().collect::<Vec<_>>(),
            ["pair", "pair", "pair", "pair"]
        );
        strings.set_range(1..2, &[c"héllo"])?;
        assert!(strings.set_range(0..2, &[c"short"]).is_err());
        assert!(strings.set_indexed(&[c"only"], &[0, 1]).is_err());
        strings.set_indexed(&[c"left", c"right"], &[0, 1])?;

        input.commit()?;
        input.node.cook_blocking()?;
        let strings = input
            .get_string_attribute::<Fixed>(0, AttributeOwner::Point, c"name")?
            .unwrap();
        assert_eq!(
            strings.get()?.iter_str().collect::<Vec<_>>(),
            ["left", "right"]
        );
        // A non-ASCII range value exercises tuple-count rather than byte-count
        // semantics independently of the indexed overwrite above.
        strings.set_range(1..2, &[c"héllo"])?;
        input.commit()?;
        input.node.cook_blocking()?;
        let strings = input
            .get_string_attribute::<Fixed>(0, AttributeOwner::Point, c"name")?
            .unwrap();
        assert_eq!(
            strings.get_range(1..2)?.iter_str().collect::<Vec<_>>(),
            ["héllo"]
        );

        let dictionary = input
            .get_dictionary_attribute::<Fixed>(0, AttributeOwner::Detail, c"data")?
            .unwrap();
        let value = dictionary.get_range(0..1)?;
        let compact: String = value
            .iter_str()
            .next()
            .unwrap()
            .split_whitespace()
            .collect();
        assert_eq!(compact, "{\"value\":1}");
        dictionary.set_range(0..1, &[c"{\"value\":2}"])?;
        input.commit()?;
        input.node.cook_blocking()?;
        let dictionary = input
            .get_dictionary_attribute::<Fixed>(0, AttributeOwner::Detail, c"data")?
            .unwrap();
        let value = dictionary.get_range(0..1)?;
        let compact: String = value
            .iter_str()
            .next()
            .unwrap()
            .split_whitespace()
            .collect();
        assert_eq!(compact, "{\"value\":2}");
        Ok(())
    })
}

#[test]
fn owner_agnostic_lookup_detects_ambiguity() -> Result<()> {
    with_session(|session| {
        let geo = session.create_input_node("ambiguous_attributes", None)?;
        let part = PartInfo::default()
            .with_part_type(PartType::Mesh)
            .with_point_count(1);
        geo.set_part_info(&part)?;
        let point_info = AttributeInfo::default()
            .with_count(1)
            .with_tuple_size(3)
            .with_owner(AttributeOwner::Point);
        geo.add_numeric_attribute::<f32, Fixed>("P", 0, point_info)?
            .set(&[0.0, 0.0, 0.0])?;
        let shared_point_info = AttributeInfo::default()
            .with_count(1)
            .with_tuple_size(1)
            .with_owner(AttributeOwner::Point);
        geo.add_numeric_attribute::<f32, Fixed>("shared", 0, shared_point_info)?
            .set(&[0.0])?;
        let detail_info = AttributeInfo::default()
            .with_count(1)
            .with_tuple_size(1)
            .with_owner(AttributeOwner::Detail);
        geo.add_numeric_attribute::<f32, Fixed>("shared", 0, detail_info)?
            .set(&[1.0])?;
        geo.commit()?;
        geo.node.cook_blocking()?;

        assert!(
            geo.get_attribute(0, AttributeOwner::Point, c"shared")?
                .is_some()
        );
        assert!(
            geo.get_attribute(0, AttributeOwner::Detail, c"shared")?
                .is_some()
        );
        assert!(geo.find_attribute(0, c"shared").is_err());
        assert!(geo.find_attribute(0, c"missing")?.is_none());
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
        let mut stale = temp.clone();
        let cached_storage = stale.info().storage();
        temp.delete()?;
        assert!(stale.refresh().is_err());
        assert_eq!(stale.info().storage(), cached_storage);
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
