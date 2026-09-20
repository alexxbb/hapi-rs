use super::*;
use crate::errors::ErrorContext;
use crate::session::{JobStatus, Session};
use std::any::Any;

type Finish<R> = Box<dyn FnOnce(Box<dyn Any + Send>) -> Result<R> + Send>;

/// An in-flight HAPI attribute operation that owns all FFI backing storage.
///
/// Call [`wait`](Self::wait) to consume the job and obtain its result. If a job
/// is dropped after HAPI has become idle, its storage is released normally. If
/// it is still running (or its status cannot be read), only the backing
/// allocation is intentionally leaked to prevent HAPI from dereferencing freed
/// memory.
#[must_use = "dropping a running HAPI job may leak its FFI backing allocation"]
pub struct AsyncJob<R> {
    job_id: i32,
    session: Session,
    backing: Option<Box<dyn Any + Send>>,
    finish: Option<Finish<R>>,
}

impl<R> std::fmt::Debug for AsyncJob<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncJob")
            .field("job_id", &self.job_id)
            .finish_non_exhaustive()
    }
}

impl<R> AsyncJob<R> {
    fn new<B: Any + Send>(
        job_id: i32,
        session: Session,
        backing: B,
        finish: impl FnOnce(B) -> Result<R> + Send + 'static,
    ) -> Self {
        Self {
            job_id,
            session,
            backing: Some(Box::new(backing)),
            finish: Some(Box::new(move |value| {
                let value = value.downcast::<B>().map_err(|_| {
                    HapiError::Internal("invalid async attribute backing storage".into())
                })?;
                finish(*value)
            })),
        }
    }

    /// Returns the HAPI job id.
    #[must_use]
    pub fn job_id(&self) -> i32 {
        self.job_id
    }

    /// Returns whether HAPI reports that the job is idle and ready to finish.
    pub fn is_ready(&self) -> Result<bool> {
        self.session
            .get_job_status(self.job_id)
            .with_context(|| format!("getting async attribute job {} status", self.job_id))
            .map(|status| status == JobStatus::Idle)
    }

    /// Waits for completion, consumes the backing storage, and returns the result.
    pub fn wait(mut self) -> Result<R> {
        while !self.is_ready()? {
            std::thread::yield_now();
        }
        let backing = self.backing.take().expect("async backing storage");
        self.finish.take().expect("async result finalizer")(backing)
    }
}

impl<R> Drop for AsyncJob<R> {
    fn drop(&mut self) {
        if self.backing.is_none() {
            return;
        }
        match self.is_ready() {
            Ok(true) => {}
            Ok(false) | Err(_) => {
                log::warn!(
                    "async attribute job {} dropped while active; leaking only its FFI backing allocation",
                    self.job_id
                );
                if let Some(backing) = self.backing.take() {
                    std::mem::forget(backing);
                }
            }
        }
    }
}

/// Feature-gated asynchronous whole-attribute reads and writes.
///
/// This extension trait is implemented for numeric handles of both shapes.
/// String-handle reads remain synchronous so their HAPI handles can be
/// resolved immediately and safely.
pub trait AsyncAttributeAccess {
    /// Value returned after a read job completes.
    type Output;
    /// Borrowed value accepted when starting a write job.
    type Input: ?Sized;
    /// Starts a read of the complete attribute.
    fn get_async(&self) -> Result<AsyncJob<Self::Output>>;
    /// Starts a write of the complete attribute.
    ///
    /// Input lengths are validated before HAPI is called.
    fn set_async(&self, values: &Self::Input) -> Result<AsyncJob<()>>;
}

/// Feature-gated asynchronous whole-attribute writes for strings and dictionaries.
///
/// The job owns every `CString` and pointer array used by HAPI until it is
/// completed or its backing allocation is deliberately leaked on early drop.
pub trait AsyncStringAttributeWrite {
    /// Borrowed value accepted when starting a write job.
    type Input: ?Sized;
    /// Starts a write of the complete attribute.
    fn set_async(&self, values: &Self::Input) -> Result<AsyncJob<()>>;
}

/// Feature-gated asynchronous unique-value writes for fixed attributes.
pub trait AsyncFixedAttributeAccess<V: ?Sized> {
    /// Starts a job assigning the same tuple value to every element.
    fn set_unique_async(&self, value: &V) -> Result<AsyncJob<()>>;
}

/// Feature-gated asynchronous indexed writes for fixed string attributes.
pub trait AsyncStringAttributeAccess {
    /// Starts a job using `indices` to select from the unique `values` table.
    ///
    /// `indices` must contain `count * tuple_size` entries.
    fn set_indexed_async<V: AsRef<CStr>>(
        &self,
        values: &[V],
        indices: &[i32],
    ) -> Result<AsyncJob<()>>;
}

struct FixedNumeric<T> {
    handle: Handle,
    info: AttributeInfo,
    data: Vec<T>,
}
struct JaggedNumeric<T> {
    handle: Handle,
    info: AttributeInfo,
    data: Vec<T>,
    sizes: Vec<i32>,
}

impl<T: NumericPrimitive> AsyncAttributeAccess for Attribute<T, Fixed> {
    type Output = Vec<T>;
    type Input = [T];

    fn get_async(&self) -> Result<AsyncJob<Vec<T>>> {
        let info = self.handle.info.clone();
        let data = vec![T::default(); Handle::expected_fixed_len(&info)?];
        let mut backing = Box::new(FixedNumeric {
            handle: self.handle.clone(),
            info,
            data,
        });
        let job = T::get_fixed_async(
            &backing.handle.node,
            backing.handle.part_id,
            &backing.handle.name,
            &mut backing.info.0,
            &mut backing.data,
        )?;
        Ok(AsyncJob::new(
            job,
            backing.handle.node.session.clone(),
            *backing,
            |b| Ok(b.data),
        ))
    }

    fn set_async(&self, values: &[T]) -> Result<AsyncJob<()>> {
        let info = self.handle.info.clone();
        let expected = Handle::expected_fixed_len(&info)?;
        if values.len() != expected {
            return Err(HapiError::Internal(format!(
                "attribute {:?} needs {expected} values, got {}",
                self.handle.name,
                values.len()
            )));
        }
        let backing = Box::new(FixedNumeric {
            handle: self.handle.clone(),
            info,
            data: values.to_vec(),
        });
        let job = T::set_fixed_async(
            &backing.handle.node,
            backing.handle.part_id,
            &backing.handle.name,
            &backing.info.0,
            &backing.data,
        )?;
        Ok(AsyncJob::new(
            job,
            backing.handle.node.session.clone(),
            *backing,
            |_| Ok(()),
        ))
    }
}

impl<T: NumericPrimitive> AsyncFixedAttributeAccess<[T]> for Attribute<T, Fixed> {
    fn set_unique_async(&self, value: &[T]) -> Result<AsyncJob<()>> {
        let info = self.handle.info.clone();
        if value.len() != usize::try_from(info.tuple_size()).unwrap_or(usize::MAX) {
            return Err(HapiError::Internal("unique tuple length mismatch".into()));
        }
        let backing = Box::new(FixedNumeric {
            handle: self.handle.clone(),
            info,
            data: value.to_vec(),
        });
        let job = T::set_unique_async(
            &backing.handle.node,
            backing.handle.part_id,
            &backing.handle.name,
            &backing.info.0,
            &backing.data,
        )?;
        Ok(AsyncJob::new(
            job,
            backing.handle.node.session.clone(),
            *backing,
            |_| Ok(()),
        ))
    }
}

impl<T: NumericPrimitive> AsyncAttributeAccess for Attribute<T, Jagged> {
    type Output = JaggedArrayData<T>;
    type Input = JaggedArrayData<T>;

    fn get_async(&self) -> Result<AsyncJob<Self::Output>> {
        let info = self.handle.info.clone();
        let data = vec![
            T::default();
            usize::try_from(info.total_array_elements())
                .map_err(|_| HapiError::Internal("negative jagged count".into()))?
        ];
        let sizes = vec![
            0;
            usize::try_from(info.count())
                .map_err(|_| HapiError::Internal("negative attribute count".into()))?
        ];
        let mut backing = Box::new(JaggedNumeric {
            handle: self.handle.clone(),
            info,
            data,
            sizes,
        });
        let job = T::get_jagged_async(
            &backing.handle.node,
            backing.handle.part_id,
            &backing.handle.name,
            &mut backing.info.0,
            &mut backing.data,
            &mut backing.sizes,
        )?;
        Ok(AsyncJob::new(
            job,
            backing.handle.node.session.clone(),
            *backing,
            |b| JaggedArrayData::from_hapi(b.data, b.sizes),
        ))
    }

    fn set_async(&self, values: &JaggedArrayData<T>) -> Result<AsyncJob<()>> {
        let info = self.handle.info.clone();
        if values.sizes().len() != usize::try_from(info.count()).unwrap_or(usize::MAX) {
            return Err(HapiError::Internal("jagged size count mismatch".into()));
        }
        let backing = Box::new(JaggedNumeric {
            handle: self.handle.clone(),
            info,
            data: values.data().to_vec(),
            sizes: values.sizes().to_vec(),
        });
        let job = T::set_jagged_async(
            &backing.handle.node,
            backing.handle.part_id,
            &backing.handle.name,
            &backing.info.0,
            &backing.data,
            &backing.sizes,
        )?;
        Ok(AsyncJob::new(
            job,
            backing.handle.node.session.clone(),
            *backing,
            |_| Ok(()),
        ))
    }
}

struct FixedStrings {
    handle: Handle,
    info: AttributeInfo,
    values: Vec<CString>,
    ptrs: Vec<*const i8>,
}
unsafe impl Send for FixedStrings {}
struct JaggedStrings {
    handle: Handle,
    info: AttributeInfo,
    values: Vec<CString>,
    ptrs: Vec<*const i8>,
    sizes: Vec<i32>,
}
unsafe impl Send for JaggedStrings {}

macro_rules! async_string_access {
    ($type:ident, $dict:expr) => {
        impl AsyncStringAttributeWrite for $type<Fixed> {
            type Input = [CString];
            fn set_async(&self, values: &[CString]) -> Result<AsyncJob<()>> {
                let info = self.handle.info.clone();
                if values.len() != Handle::expected_fixed_len(&info)? {
                    return Err(HapiError::Internal("string value count mismatch".into()));
                }
                let mut backing = Box::new(FixedStrings {
                    handle: self.handle.clone(),
                    info,
                    values: values.to_vec(),
                    ptrs: vec![],
                });
                backing.ptrs = backing.values.iter().map(|v| v.as_ptr()).collect();
                let job = crate::ffi::set_string_attribute_data_async(
                    &backing.handle.node,
                    backing.handle.part_id,
                    &backing.handle.name,
                    &backing.info.0,
                    &backing.ptrs,
                    $dict,
                )?;
                Ok(AsyncJob::new(
                    job,
                    backing.handle.node.session.clone(),
                    *backing,
                    |_| Ok(()),
                ))
            }
        }
    };
}
async_string_access!(StringAttribute, false);
async_string_access!(DictionaryAttribute, true);

macro_rules! async_jagged_string_access {
    ($type:ident, $dict:expr) => {
        impl AsyncStringAttributeWrite for $type<Jagged> {
            type Input = (Vec<CString>, Vec<i32>);
            fn set_async(&self, input: &(Vec<CString>, Vec<i32>)) -> Result<AsyncJob<()>> {
                JaggedArrayData::new(vec![(); input.0.len()], input.1.clone())?;
                let info = self.handle.info.clone();
                if input.1.len() != usize::try_from(info.count()).unwrap_or(usize::MAX) {
                    return Err(HapiError::Internal(
                        "jagged string size count mismatch".into(),
                    ));
                }
                let mut backing = Box::new(JaggedStrings {
                    handle: self.handle.clone(),
                    info,
                    values: input.0.clone(),
                    ptrs: vec![],
                    sizes: input.1.clone(),
                });
                backing.ptrs = backing.values.iter().map(|v| v.as_ptr()).collect();
                let job = crate::ffi::set_string_jagged_attribute_data_async(
                    &backing.handle.node,
                    backing.handle.part_id,
                    &backing.handle.name,
                    &backing.info.0,
                    &backing.ptrs,
                    &backing.sizes,
                    $dict,
                )?;
                Ok(AsyncJob::new(
                    job,
                    backing.handle.node.session.clone(),
                    *backing,
                    |_| Ok(()),
                ))
            }
        }
    };
}
async_jagged_string_access!(StringAttribute, false);
async_jagged_string_access!(DictionaryAttribute, true);

impl AsyncFixedAttributeAccess<CStr> for StringAttribute<Fixed> {
    fn set_unique_async(&self, value: &CStr) -> Result<AsyncJob<()>> {
        let info = self.handle.info.clone();
        let mut backing = Box::new(FixedStrings {
            handle: self.handle.clone(),
            info,
            values: vec![value.to_owned()],
            ptrs: vec![],
        });
        backing.ptrs.push(backing.values[0].as_ptr());
        let job = crate::ffi::set_string_unique_attribute_data_async(
            &backing.handle.node,
            backing.handle.part_id,
            &backing.handle.name,
            &backing.info.0,
            &backing.values[0],
        )?;
        Ok(AsyncJob::new(
            job,
            backing.handle.node.session.clone(),
            *backing,
            |_| Ok(()),
        ))
    }
}

impl AsyncStringAttributeAccess for StringAttribute<Fixed> {
    fn set_indexed_async<V: AsRef<CStr>>(
        &self,
        values: &[V],
        indices: &[i32],
    ) -> Result<AsyncJob<()>> {
        let info = self.handle.info.clone();
        if indices.len() != Handle::expected_fixed_len(&info)? {
            return Err(HapiError::Internal("string index count mismatch".into()));
        }
        validate_string_indices(values.len(), indices)?;
        let mut backing = Box::new(JaggedStrings {
            handle: self.handle.clone(),
            info,
            values: values.iter().map(|v| v.as_ref().to_owned()).collect(),
            ptrs: vec![],
            sizes: indices.to_vec(),
        });
        backing.ptrs = backing.values.iter().map(|v| v.as_ptr()).collect();
        let job = crate::ffi::set_indexed_string_attribute_data_async(
            &backing.handle.node,
            backing.handle.part_id,
            &backing.handle.name,
            &backing.info.0,
            &backing.ptrs,
            &backing.sizes,
        )?;
        Ok(AsyncJob::new(
            job,
            backing.handle.node.session.clone(),
            *backing,
            |_| Ok(()),
        ))
    }
}
