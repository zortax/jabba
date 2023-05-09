use std::future::Future;
use std::pin::Pin;
use async_lock::{Mutex, OnceCell};

pub(crate) type DynError = Box<dyn std::error::Error + Send + Sync>;

pub(crate) struct LazyCell<T: Send + Sync + 'static> {
  cell: OnceCell<T>,
  future: Mutex<
    Option<Pin<Box<dyn Future<Output = Result<T, DynError>> + Sync + Send>>>,
  >,
}

impl<T: Send + Sync + 'static> LazyCell<T> {
  pub fn new(
    future: impl Future<Output = Result<T, DynError>> + Send + Sync + 'static,
  ) -> Self {
    Self {
      cell: OnceCell::new(),
      future: Mutex::new(Some(Box::pin(future))),
    }
  }

  pub async fn get(&self) -> Result<&T, DynError> {
    let mut generator_guard = self.future.lock().await;
    let generator = generator_guard
      .take()
      .unwrap_or_else(|| Box::pin(async { unreachable!() }));
    self.cell.get_or_try_init(|| generator).await
  }
}
