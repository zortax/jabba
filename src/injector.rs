use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::marker::Unsize;
use std::mem;
use std::pin::Pin;
use std::sync::Arc;

use async_lock::RwLock;
use async_trait::async_trait;

use crate::error::Error;
use crate::lazy_cell::LazyCell;
use crate::{Injectable, Singleton};

#[derive(Debug, Eq, PartialEq, Hash)]
enum BindingKey {
  Named(TypeId, String),
  Unnamed(TypeId),
}

enum Binding {
  Singleton(LazyCell<Box<dyn Any + Send + Sync + 'static>>),
  Constructor(
    Box<
      dyn Fn() -> Pin<
          Box<
            dyn Future<
                Output = Result<
                  Box<dyn Any + Send + Sync>,
                  Box<dyn std::error::Error + Send + Sync>,
                >,
              > + Sync
              + Send,
          >,
        > + Send
        + Sync,
    >,
  ),
}

#[async_trait]
trait SpecializationBinder<T: ?Sized, I> {
  async fn bind_internal(&self, key: BindingKey);
}

#[derive(Clone)]
pub struct Injector {
  inner: Arc<InjectorInner>,
}

struct InjectorInner {
  bindings: RwLock<HashMap<BindingKey, Binding>>,
}

impl Injector {
  pub fn new() -> Self {
    Self {
      inner: Arc::new(InjectorInner {
        bindings: RwLock::new(HashMap::new()),
      }),
    }
  }

  async fn get_internal<T: ?Sized + Injectable + 'static>(
    &self,
    key: BindingKey,
  ) -> Result<Arc<T>, Error<T::Error>> {
    if let Some(binding) = self.inner.bindings.read().await.get(&key) {
      match binding {
        Binding::Singleton(cell) => match cell.get().await {
          Ok(instance) => {
            let instance: &Box<Arc<T>> = unsafe {
              mem::transmute::<
                &Box<dyn Any + Sync + Send + 'static>,
                &Box<Arc<T>>,
              >(instance)
            };
            Ok(*instance.clone())
          }
          Err(dyn_error) => {
            let error: Box<T::Error> =
              dyn_error.downcast::<T::Error>().unwrap();
            Err(Error::InstanceCreationFailed(*error))
          }
        },
        Binding::Constructor(constructor) => {
          match Box::pin(constructor()).await {
            Ok(instance) => {
              let instance = instance.downcast::<Arc<T>>().unwrap();
              Ok(*instance)
            }
            Err(dyn_error) => {
              let error: Box<T::Error> =
                dyn_error.downcast::<T::Error>().unwrap();
              Err(Error::InstanceCreationFailed(*error))
            }
          }
        }
      }
    } else {
      Err(Error::NoBinding)
    }
  }

  pub async fn bind<
    T: ?Sized + Sync + Send + 'static,
    I: Unsize<T> + Injectable + 'static,
  >(
    &self,
  ) {
    let fut = (self as &dyn SpecializationBinder<T, I>)
      .bind_internal(BindingKey::Unnamed(TypeId::of::<T>()));
    fut.await;
  }

  pub async fn bind_named<
    T: ?Sized + Sync + Send + 'static,
    I: Unsize<T> + Singleton + Injectable + 'static,
  >(
    &self,
    name: impl ToString + 'static,
  ) {
    let fut = (self as &dyn SpecializationBinder<T, I>)
      .bind_internal(BindingKey::Named(TypeId::of::<T>(), name.to_string()));
    fut.await;
  }

  pub fn bind_cloneable<
    T: ?Sized + Sync + 'static,
    I: Unsize<T> + Clone + Injectable + 'static,
  >(
    &self,
  ) {
    todo!()
  }

  pub fn get<T: ?Sized + Injectable + 'static>(
    &self,
  ) -> impl Future<Output = Result<Arc<T>, Error<T::Error>>> + Send + Sync + '_
  {
    async {
      self
        .get_internal(BindingKey::Unnamed(TypeId::of::<T>()))
        .await
    }
  }

  pub async fn get_named<T: ?Sized + Injectable + 'static>(
    &self,
    name: impl ToString,
  ) -> Result<Arc<T>, Error<T::Error>> {
    self
      .get_internal(BindingKey::Named(TypeId::of::<T>(), name.to_string()))
      .await
  }

  pub async fn get_box<T: ?Sized + Injectable + 'static>(
    &self,
  ) -> Result<Box<T>, Error<T::Error>> {
    todo!()
  }
}

#[async_trait]
impl<T, I> SpecializationBinder<T, I> for Injector
where
  T: ?Sized + Sync + Send + 'static,
  I: Unsize<T> + Injectable + 'static,
{
  default async fn bind_internal(&self, _key: BindingKey) {
    let type_id = TypeId::of::<T>();
    let injector = self.clone();

    self.inner.bindings.write().await.insert(
      BindingKey::Unnamed(type_id),
      Binding::Constructor(Box::new(move || {
        let injector = injector.clone();
        Box::pin(async move {
          match I::create(injector).await {
            Ok(instance) => Ok(Box::new(Arc::new(instance) as Arc<T>)
              as Box<dyn Any + Sync + Send>),
            Err(err) => {
              Err(Box::new(err) as Box<dyn std::error::Error + Send + Sync>)
            }
          }
        })
      })),
    );
  }
}

#[async_trait]
impl<T, I> SpecializationBinder<T, I> for Injector
where
  T: ?Sized + Sync + Send + 'static,
  I: Unsize<T> + Injectable + Singleton + 'static,
{
  default async fn bind_internal(&self, key: BindingKey) {
    let generator = I::create(self.clone());
    let inner = self.inner.clone();

    let cell: LazyCell<Box<dyn Any + Sync + Send + 'static>> =
      LazyCell::new(Box::pin(async move {
        Ok(Box::new(Arc::new(generator.await?) as Arc<T>)
          as Box<dyn Any + Sync + Send>)
      }));

    inner
      .bindings
      .write()
      .await
      .insert(key, Binding::Singleton(cell));
  }
}
