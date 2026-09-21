#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! AIDD 对抗 / 边界用例（特性 002）。
//!
//! 候选由 AI 生成，逐条人工复核后仅保留「结论=保留」项；丢弃项登记于 PR 描述。
//!
//! // AIDD: op 含控制字符与换行 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §3 防日志注入 | 结论=保留
//! // AIDD: op 超长（10 万字符） | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §3 长度限制义务 | 结论=保留
//! // AIDD: attempt 取 u32::MAX | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §2 实现方不得假设起点 | 结论=保留
//! // AIDD: 空字符串 op | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=契约不禁止空 op | 结论=保留
//! // AIDD: 多线程并发调用三方法 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §3 Send+Sync | 结论=保留
//! // AIDD: Unicode 与组合字符 op | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §3 清理义务 | 结论=保留

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use instrumentationx::Instrumentation;

#[derive(Debug, Default)]
struct Sink {
    retries: AtomicU64,
    opens: AtomicU64,
    closes: AtomicU64,
}

impl Instrumentation for Sink {
    fn record_retry(&self, _op: &str, _attempt: u32) {
        self.retries.fetch_add(1, Ordering::Relaxed);
    }
    fn record_circuit_open(&self, _op: &str) {
        self.opens.fetch_add(1, Ordering::Relaxed);
    }
    fn record_circuit_close(&self, _op: &str) {
        self.closes.fetch_add(1, Ordering::Relaxed);
    }
}

/// 边界：op 含控制字符与换行——实现不得 panic、不得丢计数。
#[test]
fn op_with_control_chars_and_newlines() {
    let s = Sink::default();
    let dyn_ref: &dyn Instrumentation = &s;
    dyn_ref.record_retry("op\n\r\x00\x1b[31m", 1);
    assert_eq!(s.retries.load(Ordering::Relaxed), 1);
}

/// 边界：op 超长（10 万字符）——同上，不得 panic。
#[test]
fn op_extremely_long() {
    let s = Sink::default();
    let dyn_ref: &dyn Instrumentation = &s;
    let long = "a".repeat(100_000);
    dyn_ref.record_circuit_open(&long);
    assert_eq!(s.opens.load(Ordering::Relaxed), 1);
}

/// 边界：attempt 取 u32 极值——契约不限制取值域，透传即可。
#[test]
fn attempt_at_u32_max() {
    let s = Sink::default();
    let dyn_ref: &dyn Instrumentation = &s;
    dyn_ref.record_retry("op", u32::MAX);
    dyn_ref.record_retry("op", 0);
    assert_eq!(s.retries.load(Ordering::Relaxed), 2);
}

/// 边界：空字符串 op——契约不禁止，实现照常计数。
#[test]
fn empty_op() {
    let s = Sink::default();
    let dyn_ref: &dyn Instrumentation = &s;
    dyn_ref.record_circuit_close("");
    assert_eq!(s.closes.load(Ordering::Relaxed), 1);
}

/// 边界：多线程并发调用三方法——计数无丢失（Relaxed 计数只保证最终一致，
/// 每线程固定调用次数下总和必须精确）。
#[test]
fn concurrent_calls_across_threads() {
    let s = Arc::new(Sink::default());
    let mut workers = Vec::new();
    for _ in 0..8 {
        let shared: Arc<dyn Instrumentation> = s.clone();
        workers.push(std::thread::spawn(move || {
            for _ in 0..100 {
                shared.record_retry("op", 1);
                shared.record_circuit_open("op");
                shared.record_circuit_close("op");
            }
        }));
    }
    for w in workers {
        w.join().expect("工作线程不应 panic");
    }
    assert_eq!(s.retries.load(Ordering::Relaxed), 800);
    assert_eq!(s.opens.load(Ordering::Relaxed), 800);
    assert_eq!(s.closes.load(Ordering::Relaxed), 800);
}

/// 边界：Unicode / 组合字符 / 全角点 op——实现不得因非 ASCII panic。
#[test]
fn unicode_op() {
    let s = Sink::default();
    let dyn_ref: &dyn Instrumentation = &s;
    dyn_ref.record_retry("数据库．查询\u{0301}", 1);
    dyn_ref.record_circuit_open("数据库．查询\u{0301}");
    assert_eq!(s.retries.load(Ordering::Relaxed), 1);
    assert_eq!(s.opens.load(Ordering::Relaxed), 1);
}
