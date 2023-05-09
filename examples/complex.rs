#![allow(incomplete_features)]
#![feature(async_fn_in_trait)]

use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use jabba::{Infallible, Injectable, Injector};
use thiserror::Error;

trait TestTrait: Injectable<Error = Infallible> + Sync + Send {
  fn value(&self) -> usize;
}

struct TestStruct {
  value: usize,
}

impl TestTrait for TestStruct {
  fn value(&self) -> usize {
    self.value
  }
}

impl Default for TestStruct {
  fn default() -> Self {
    static COUNTER: AtomicUsize = AtomicUsize::new(42);
    TestStruct {
      value: COUNTER.fetch_add(1, Ordering::SeqCst),
    }
  }
}

#[derive(Debug, Error)]
enum InjectionDemoError {
  #[error("Failed getting instance from injector: {0}")]
  InjectionError(#[from] jabba::Error<Infallible>),

  #[allow(dead_code)]
  #[error("Other error variant")]
  OtherError,
}

#[async_trait]
trait InjectionDemoTrait:
  Injectable<Error = InjectionDemoError> + Sync + Send
{
  fn value(&self) -> usize;

  async fn test(&self);
}

struct InjectionDemoStruct {
  value: usize,
}

#[async_trait]
impl InjectionDemoTrait for InjectionDemoStruct {
  fn value(&self) -> usize {
    self.value
  }

  async fn test(&self) {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    println!("test");
  }
}

impl Injectable for InjectionDemoStruct {
  type Error = InjectionDemoError;

  async fn create(injector: Injector) -> Result<Self, Self::Error> {
    let other_instance = injector.get::<dyn TestTrait>().await?;

    Ok(Self {
      value: other_instance.value(),
    })
  }
}

#[tokio::main]
async fn main() {
  let injector = Injector::new();

  injector.bind::<dyn TestTrait, TestStruct>().await;
  injector
    .bind::<dyn InjectionDemoTrait, InjectionDemoStruct>()
    .await;

  let instance1 = injector.get::<dyn InjectionDemoTrait>().await.unwrap();
  println!("instance1 value: {}", instance1.value());

  let instance2 = injector.get::<dyn InjectionDemoTrait>().await.unwrap();

  instance2.test().await;

  println!("instance2 value: {}", instance2.value());
}
