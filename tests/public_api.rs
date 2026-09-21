#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]

//! 公共 API 表面：`instrumentationx::Instrumentation` 可从 crate 外导入、
//! 对象安全、`Send + Sync`，且能作为 trait object 在多线程间共享。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use instrumentationx::Instrumentation;

fn assert_send_sync<T: Send + Sync + ?Sized>() {}

#[derive(Debug, Default)]
struct CountingInstrumentation {
    retries: AtomicU64,
    opens: AtomicU64,
    closes: AtomicU64,
    last_attempt: AtomicU64,
}

impl Instrumentation for CountingInstrumentation {
    fn record_retry(&self, _op: &str, attempt: u32) {
        self.retries.fetch_add(1, Ordering::Relaxed);
        self.last_attempt
            .store(u64::from(attempt), Ordering::Relaxed);
    }
    fn record_circuit_open(&self, _op: &str) {
        self.opens.fetch_add(1, Ordering::Relaxed);
    }
    fn record_circuit_close(&self, _op: &str) {
        self.closes.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn trait_object_is_send_and_sync() {
    assert_send_sync::<dyn Instrumentation>();
    assert_send_sync::<CountingInstrumentation>();
}

#[test]
fn consumer_can_inject_any_implementation_through_dyn() {
    fn drive(instrumentation: &dyn Instrumentation) {
        instrumentation.record_retry("db.query", 1);
        instrumentation.record_circuit_open("db.query");
        instrumentation.record_circuit_close("db.query");
    }

    let counter = CountingInstrumentation::default();
    drive(&counter);
    assert_eq!(counter.retries.load(Ordering::Relaxed), 1);
    assert_eq!(counter.opens.load(Ordering::Relaxed), 1);
    assert_eq!(counter.closes.load(Ordering::Relaxed), 1);
    assert_eq!(counter.last_attempt.load(Ordering::Relaxed), 1);
}

#[test]
fn trait_object_can_be_shared_across_threads() {
    let counter = Arc::new(CountingInstrumentation::default());
    let shared: Arc<dyn Instrumentation> = counter.clone();

    let workers: Vec<_> = (0..4)
        .map(|worker| {
            let shared = Arc::clone(&shared);
            std::thread::spawn(move || shared.record_retry("worker.op", worker + 1))
        })
        .collect();
    for worker in workers {
        worker.join().expect("工作线程不应 panic");
    }

    assert_eq!(counter.retries.load(Ordering::Relaxed), 4);
}

#[test]
fn impl_on_reference_and_box_are_usable() {
    // 引用与 Box 都应能充当 trait object，便于注入方按需选择所有权形式。
    let counter = CountingInstrumentation::default();
    let by_ref: &dyn Instrumentation = &counter;
    by_ref.record_retry("by.ref", 2);

    let boxed: Box<dyn Instrumentation> = Box::new(CountingInstrumentation::default());
    boxed.record_circuit_open("by.box");

    assert_eq!(counter.retries.load(Ordering::Relaxed), 1);
    assert_eq!(counter.last_attempt.load(Ordering::Relaxed), 2);
}
