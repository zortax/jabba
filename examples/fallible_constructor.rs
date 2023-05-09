#![allow(incomplete_features)]
#![feature(async_fn_in_trait)]

use std::sync::atomic::{AtomicUsize, Ordering};

use jabba::{Injectable, Injector};
use thiserror::Error;

#[derive(Error, Debug)]
enum CustomError {
  #[error("Error variant A")]
  VariantA,

  #[allow(dead_code)]
  #[error("Error variant B")]
  VariantB,
}

trait TestTrait: Injectable<Error = CustomError> + Send {
  fn value(&self) -> usize;
}

#[derive(Debug)]
struct TestStruct {
  value: usize,
}

impl TestTrait for TestStruct {
  fn value(&self) -> usize {
    self.value
  }
}

impl Injectable for TestStruct {
  type Error = CustomError;

  async fn create(_: Injector) -> Result<Self, Self::Error> {
    static COUNTER: AtomicUsize = AtomicUsize::new(42);

    let value = COUNTER.fetch_add(1, Ordering::SeqCst);

    if value == 42 {
      Ok(Self { value })
    } else {
      Err(CustomError::VariantA)
    }
  }
}

#[tokio::main]
async fn main() {
  let injector = Injector::new();

  injector.bind::<dyn TestTrait, TestStruct>().await;

  let instance1 = injector.get::<dyn TestTrait>().await.unwrap();
  println!("instance1 value: {}", instance1.value());

  match injector.get::<dyn TestTrait>().await {
    Ok(instance2) => println!("instance2 value: {}", instance2.value()),
    Err(err) => println!("Error: {}", err),
  }
}
