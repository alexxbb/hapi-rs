use crate::{
    errors::Result,
};
use super::parameter::{
    ParmNodeWrap, ParmValue,
};

pub trait ParmBaseTrait<'s> {
    type ReturnType;

    fn wrap(&self) -> &ParmNodeWrap<'s>;
    fn array_index(&self) -> i32;
    fn values_array(&self) -> Result<Vec<Self::ReturnType>>;
}

pub trait ParameterTrait<'s>: ParmBaseTrait<'s> {
    fn name(&self) -> Result<String>;
    fn get_value(&self) -> Result<ParmValue<<Self as ParmBaseTrait<'s>>::ReturnType>>;
}


impl<'s, T> ParameterTrait<'s> for T
    where
        T: ParmBaseTrait<'s>,
{
    fn name(&self) -> Result<String> {
        let wrap = self.wrap();
        match wrap.info.name.as_ref() {
            None => wrap.info.name(),
            Some(n) => Ok(n.to_string_lossy().to_string()),
        }
    }

    fn get_value(&self) -> Result<ParmValue<<T as ParmBaseTrait<'s>>::ReturnType>> {
        let wrap = self.wrap();
        let size = wrap.info.size();
        let mut values = self.values_array()?;
        debug_assert_eq!(values.len(), size as usize);
        Ok(match size {
            1 => ParmValue::Single(values.pop().unwrap()),
            2 => ParmValue::Tuple2((values.remove(0), values.remove(0))),
            3 => ParmValue::Tuple3((values.remove(0), values.remove(0), values.remove(0))),
            4 => ParmValue::Tuple4((
                values.remove(0),
                values.remove(0),
                values.remove(0),
                values.remove(0),
            )),
            _ => ParmValue::Array(values),
        })
    }
}


