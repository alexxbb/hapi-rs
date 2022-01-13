#![allow(unused)]

use std::any::Any;
use std::marker::PhantomData;

struct DataArray<T>(T);

struct NumericAttr<T> {
    _m: PhantomData<T>,
    info: Info
}

struct NumericArrayAttr<T> {
    _m: PhantomData<T>,
    info: Info
}

enum StorageType {
    Int,
    Float,
}

trait AttribStorage {
    fn storage() -> StorageType;
}
macro_rules! impl_storage {
    ($tp:ty, $st:expr) => {
        impl AttribStorage for $tp {
            fn storage() -> StorageType {
                $st
            }
        }
    };
}

impl_storage!(i32, StorageType::Int);
impl_storage!(f32, StorageType::Float);

impl<T: AttribTypeImpl> NumericArrayAttr<T> {
    fn get(&self) -> DataArray<Vec<T>> { T::get_array() }
    fn set(&self, values: &DataArray<&[T]>) { T::set_array() }
}


impl<T: AttribTypeImpl> NumericAttr<T> {
    fn get(&self) -> Vec<T> { T::get() }
    fn set(&self, values: &[T]) { T::set() }
}

trait AttribTypeImpl: Sized + AttribStorage {
    fn get() -> Vec<Self>;
    fn set();
    fn get_array() -> DataArray<Vec<Self>>;
    fn set_array();
}

#[derive(Default)]
struct Info {
    name: String
}

impl Info {
    fn new(name: &str) -> Self {
        Info{name: name.to_string()}
    }
}

trait AsAttribute {
    fn info(&self) -> &Info;
}

impl<T> AsAttribute for NumericAttr<T>{
    fn info(&self) -> &Info {
        &self.info
    }
}
impl<T> AsAttribute for NumericArrayAttr<T>{
    fn info(&self) -> &Info {
        &self.info
    }
}

impl<T: Sized + AttribStorage> AttribTypeImpl for T {
    fn get() -> Vec<T> { vec![] }
    fn set() {}
    fn get_array() -> DataArray<Vec<T>> { DataArray(vec![]) }
    fn set_array() {}
}

// object safe trait
trait AnyAttribWrapper: Any + AsAttribute {
    fn as_any(&self) -> &dyn Any;
}

impl<T: AsAttribute + 'static> AnyAttribWrapper for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct Attribute(Box<dyn AnyAttribWrapper>);

impl Attribute {
    fn new(inner: Box<dyn AnyAttribWrapper>) -> Self {
        Attribute(inner)
    }
    fn downcast<T: AnyAttribWrapper>(&self) -> Option<&T> {
        self.0.as_any().downcast_ref::<T>()
    }
    fn name(&self) -> &str {
        self.0.info().name.as_str()
    }
}

fn get_attributes() -> Vec<Attribute> {
    let a = NumericAttr::<f32> { _m: PhantomData, info: Info::new("NumericAttr a") };
    let b = NumericAttr::<i32> { _m: PhantomData, info: Info::new("NumericAttr b")  };
    let c = NumericArrayAttr::<i32> { _m: PhantomData, info: Info::new("NumericArrayAttr c") };
    vec![
        Attribute::new(Box::new(a)),
        Attribute::new(Box::new(b)),
        Attribute::new(Box::new(c)),
    ]
}

fn main() {
    for attr in &get_attributes() {
        if let Some(a) = attr.downcast::<NumericAttr<i32>>() {
            a.get();
            a.set(&[1, 2, 3]);
            println!("{}", a.info().name)
        }
        if let Some(a) = attr.downcast::<NumericArrayAttr<i32>>() {
            let da = a.get();
            a.set(&DataArray(&[1, 2, 3]));
            println!("{}", attr.name());
        }
    }
}
