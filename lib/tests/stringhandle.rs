use hapi_rs::Result;

mod utils;
use utils::with_session;

#[test]
fn get_string_api() -> Result<()> {
    with_session(|session| {
        assert!(!session.get_server_var::<str>("HFS")?.is_empty());
        Ok(())
    })
}

#[test]
fn string_array_api() -> Result<()> {
    with_session(|session| {
        session.set_server_var::<str>("TEST", "177")?;
        let array = session.get_server_variables()?;
        assert!(array.iter_str().any(|s| s == "TEST=177"));
        assert!(
            array
                .iter_cstr()
                .any(|s| s.to_bytes_with_nul() == b"TEST=177\0")
        );
        let mut owned = array.into_iter();
        assert!(owned.any(|s| s == "TEST=177"));
        let handle = session.set_custom_string("hapi-rs custom string")?;
        assert_eq!(session.get_string(handle)?, "hapi-rs custom string");
        let strings = session.get_string_batch(&[handle])?;
        assert_eq!(
            strings.into_iter().collect::<Vec<_>>(),
            vec!["hapi-rs custom string".to_string()]
        );
        session.remove_custom_string(handle)?;
        Ok(())
    })
}

#[test]
fn get_string_batch() -> Result<()> {
    with_session(|session| {
        let handle = session.set_custom_string("HAPI_RS_BATCH_TEST")?;
        let array = session.get_string_batch(&[handle])?;
        assert_eq!(
            array.iter_str().collect::<Vec<_>>(),
            vec!["HAPI_RS_BATCH_TEST"]
        );
        session.remove_custom_string(handle)?;
        Ok(())
    })
}

#[test]
fn set_custom_string() -> Result<()> {
    with_session(|session| {
        let handle = session.set_custom_string("HAPI_RS_CUSTOM_STRING")?;
        assert_eq!(session.get_string(handle)?, "HAPI_RS_CUSTOM_STRING");
        session.remove_custom_string(handle)?;
        assert!(session.get_string(handle).is_err());
        Ok(())
    })
}
