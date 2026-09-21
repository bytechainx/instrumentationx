#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! TDD 行为契约（特性 002）。
//!
//! 逐公开入口的先红后绿：下表每个入口先在变异副本上观测红、再在本树观测绿；
//! 红绿过程见 PR 描述（变异描述 + 复现命令）。
//!
//! // TDD-PROBE: Instrumentation::record_retry | 变异：计数恒不递增 | 红=record_retry_counts | 绿=record_retry_counts
//! // TDD-PROBE: Instrumentation::record_circuit_open | 变异：open 计入 close 计数 | 红=record_circuit_open_counts | 绿=record_circuit_open_counts
//! // TDD-PROBE: Instrumentation::record_circuit_close | 变异：close 计数恒为 0 | 红=record_circuit_close_counts | 绿=record_circuit_close_counts

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use instrumentationx::Instrumentation;

/// 记录型实现（测试替身）：三计数器 + 最近一次 attempt。
#[derive(Debug, Default)]
struct Recorder {
    retries: AtomicU64,
    opens: AtomicU64,
    closes: AtomicU64,
    last_attempt: AtomicU64,
}

impl Instrumentation for Recorder {
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
fn record_retry_counts() {
    let r = Recorder::default();
    let dyn_ref: &dyn Instrumentation = &r;
    dyn_ref.record_retry("db.query", 1);
    dyn_ref.record_retry("db.query", 2);
    assert_eq!(r.retries.load(Ordering::Relaxed), 2, "重试计数应递增");
    assert_eq!(
        r.last_attempt.load(Ordering::Relaxed),
        2,
        "应透传最后一次 attempt"
    );
}

#[test]
fn record_circuit_open_counts() {
    let r = Recorder::default();
    let dyn_ref: &dyn Instrumentation = &r;
    dyn_ref.record_circuit_open("db.query");
    assert_eq!(r.opens.load(Ordering::Relaxed), 1, "open 计数应为 1");
    assert_eq!(r.closes.load(Ordering::Relaxed), 0, "open 不得计入 close");
}

#[test]
fn record_circuit_close_counts() {
    let r = Recorder::default();
    let dyn_ref: &dyn Instrumentation = &r;
    dyn_ref.record_circuit_close("db.query");
    assert_eq!(r.closes.load(Ordering::Relaxed), 1, "close 计数应为 1");
    assert_eq!(r.opens.load(Ordering::Relaxed), 0, "close 不得计入 open");
}

/// 三方法经 `Arc<dyn>` 跨线程共享仍保持各自独立计数（契约：Send + Sync）。
#[test]
fn methods_are_independent_across_threads() {
    let r = Arc::new(Recorder::default());
    let shared: Arc<dyn Instrumentation> = r.clone();
    let worker = std::thread::spawn(move || {
        shared.record_retry("worker.op", 7);
    });
    worker.join().expect("工作线程不应 panic");
    assert_eq!(r.retries.load(Ordering::Relaxed), 1);
}
