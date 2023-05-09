use std::sync::atomic::{AtomicUsize, Ordering};

use jabba::{Infallible, Injectable, Injector};

trait TestTrait: Injectable<Error = Infallible> + Send {
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

#[tokio::main]
async fn main() {
  let injector = Injector::new();

  injector.bind::<dyn TestTrait, TestStruct>().await;

  let instance1 = injector.get::<dyn TestTrait>().await.unwrap();
  println!("instance1 value: {}", instance1.value());

  let instance2 = injector.get::<dyn TestTrait>().await.unwrap();
  println!("instance2 value: {}", instance2.value());
}
