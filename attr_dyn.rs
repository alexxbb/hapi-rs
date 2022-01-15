#![allow(unused)]

use std::any::Any;
use std::marker::PhantomData;

struct DataArray<T, I: IntoIterator<Item=T>>(I);

struct DataIter<I: IntoIterator> {
    iter: <I as IntoIterator>::IntoIter
}

struct DataRefIter<T, I: Iterator<Item=T>> {
    iter: I
}

// impl<T, I: IntoIterator<Item=T>> DataArray<T,I> {
//
//     fn iter(&self) -> DataRefIter<T, I::IntoIter> {
//         DataRefIter{iter: self.0.into_iter()}
//     }
//
// }

// impl<T, I> Iterator for DataArray<T, I> {
//     type Item = T;
//
//     fn next(&mut self) -> Option<Self::Item> {
//         self.
//     }
//
// }


impl<T, I: IntoIterator<Item=T>> IntoIterator for DataArray<T, I> {
    type Item = T;
    type IntoIter = DataIter<<I as IntoIterator>::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        DataIter{iter: self.0.into_iter()}
    }
}

impl<T, I: IntoIterator<Item=T>> Iterator for DataIter<I> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

}

struct NumericAttr<T> {
    _m: PhantomData<T>,
    info: Info
}

struct NumericArrayAttr<T> {
    _m: PhantomData<T>,
    info: Info
}

#[derive(Debug)]
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
    fn get(&self) -> DataArray<T, Vec<T>> { T::get_array() }
    fn set<'a>(&self, values: &'a DataArray<&'a T, &'a [T]>) { T::set_array() }
}


impl<T: AttribTypeImpl> NumericAttr<T> {
    fn get(&self) -> Vec<T> { T::get() }
    fn set(&self, values: &[T]) { T::set() }
}

trait AttribTypeImpl: Sized + AttribStorage {
    fn get() -> Vec<Self>;
    fn set();
    fn get_array() -> DataArray<Self, Vec<Self>>;
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
    fn storage(&self) -> StorageType;
}

impl<T: AttribStorage> AsAttribute for NumericAttr<T>{
    fn info(&self) -> &Info {
        &self.info
    }
    fn storage(&self) -> StorageType {
        T::storage()
    }
}
impl<T: AttribStorage> AsAttribute for NumericArrayAttr<T>{
    fn info(&self) -> &Info {
        &self.info
    }
    fn storage(&self) -> StorageType {
        T::storage()
    }
}

impl<T: Sized + AttribStorage> AttribTypeImpl for T {
    fn get() -> Vec<T> { vec![] }
    fn set() {}
    fn get_array() -> DataArray<T, Vec<T>> { DataArray(vec![]) }
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
    fn storage(&self) -> StorageType {
        self.0.storage()
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
    let dat = DataArray(&[1,3,4]);
    for v in dat.into_iter() {
        println!("{}", v);
    }
    let dat = DataArray(vec![1,2,3]);
    for v in dat.into_iter() {
        println!("{}", v);
    }
    for attr in &get_attributes() {
        println!("Storage:{:?}", attr.storage());
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

/// DataArray

struct ArrayIter<'a, T> {
    sizes: std::slice::Iter<'a, i32>,
    data: std::slice::Iter<'a, T>
}

impl<'a, T> Iterator for ArrayIter<'a ,T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.data.next()
    }
}

struct DataArray<'a, T> where [T]: ToOwned<Owned = Vec<T>>
{
    data: Cow<'a, [T]>,
    sizes: Cow<'a, [i32]>
}
impl<'a, T> DataArray<'a, T>
where
    [T]: ToOwned<Owned = Vec<T>>,
{
    fn new(dat: &'a [T], sizes: &'a [i32]) -> DataArray<'a, T> {
        DataArray { data: Cow::Borrowed(dat), sizes: Cow::Borrowed(sizes) }
    }

    pub(crate) fn new_owned(dat: Vec<T>, sizes: Vec<i32>) -> DataArray<'static, T> {
        DataArray { data: Cow::Owned(dat), sizes: Cow::Owned(sizes) }
    }

    fn data(&self) -> &[T] {
        self.data.as_ref()
    }

    fn data_mut(&mut self) -> &mut [T] {
        self.data.to_mut().as_mut()
    }
    fn sizes(&self) -> &[i32] {
        self.sizes.as_ref()
    }

    fn iter_values(&'a self) -> ArrayIter<'a, T> {
        ArrayIter{sizes: self.sizes.iter(), data: self.data.iter()}
    }
}

