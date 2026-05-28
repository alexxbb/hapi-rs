use hapi_rs::Result;
use pretty_assertions::assert_eq;

mod utils;
use utils::with_session;

#[test]
fn pdg_create_workitems() -> Result<()> {
    with_session(|session| {
        let topnet = session.create_node("Object/topnet")?;
        let generator = topnet
            .session
            .node_builder("genericgenerator")
            .with_parent(&topnet)
            .create()?
            .to_top_node()
            .ok_or_else(|| hapi_rs::HapiError::Internal("TOP node".into()))?;

        generator.node.cook_blocking()?;
        let workitem = generator.create_workitem("test_1", 0, None)?;
        workitem.set_int_data("my_int_data", &[1, 2, 3])?;
        workitem.set_float_data("my_float_data", &[1.0, 2.0, 3.0])?;
        generator.commit_workitems()?;
        generator.cook_pdg_blocking(false)?;
        let i_data = workitem.get_int_data("my_int_data")?;
        assert_eq!(&i_data, &[1, 2, 3]);
        let f_data = workitem.get_float_data("my_float_data")?;
        assert_eq!(&f_data, &[1.0, 2.0, 3.0]);
        Ok(())
    })
}
