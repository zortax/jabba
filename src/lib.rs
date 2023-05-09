#![feature(unsize)]
#![allow(incomplete_features)]
#![feature(specialization)]
#![feature(async_fn_in_trait)]
#![feature(return_position_impl_trait_in_trait)]

#[macro_use]
extern crate thiserror;

use std::future::Future;

pub use error::Error;
pub use error::Infallible;
pub use injector::Injector;

mod error;
mod injector;
mod lazy_cell;

pub trait Singleton {}

pub trait Injectable: Sync + Send {
  type Error: std::error::Error + Send + Sync + 'static;

  fn create(
    injector: Injector,
  ) -> impl Future<Output = Result<Self, Self::Error>> + Sync + Send
  where
    Self: Sized;
}

impl<T> Injectable for T
where
  T: Default + Send + Sync + 'static,
{
  type Error = Infallible;

  async fn create(_: Injector) -> Result<Self, Self::Error> {
    Ok(Default::default())
  }
}

#[cfg(test)]
mod tests {}
