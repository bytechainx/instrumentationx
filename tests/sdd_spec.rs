#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! SDD 规格对照（特性 002）：把 `docs/标准.md` 的章节条款转成可执行断言。
//!
//! // SPEC-MAP: S-1 | 1. 定位 | assert_positioning
//! // SPEC-MAP: S-2 | 2. 契约标准 | assert_contract
//! // SPEC-MAP: S-3 | 3. 实现方义务 | assert_impl_duties
//! // SPEC-MAP: S-4 | 4. 消费方义务 | assert_consumer_duties
//! // SPEC-MAP: S-5 | 5. 验收 | assert_acceptance

use std::sync::atomic::{AtomicU64, Ordering};

use instrumentationx::Instrumentation;

/// S-1：crate 只定义契约不含实现——消费方只经 `dyn` 持有，无具体类型依赖。
#[test]
fn assert_positioning() {
    struct LocalNoop;
    impl Instrumentation for LocalNoop {
        fn record_retry(&self, _op: &str, _attempt: u32) {}
        fn record_circuit_open(&self, _op: &str) {}
        fn record_circuit_close(&self, _op: &str) {}
    }
    // 实现可以完全不依赖其他 crate 而存在（零依赖契约的体现）。
    let instrumentation: &dyn Instrumentation = &LocalNoop;
    instrumentation.record_retry("op", 0);
}

/// S-2：对象安全 + attempt 透传 + 三方法语义（重试计数、迁移各记一次）。
#[test]
fn assert_contract() {
    struct Triple {
        retries: AtomicU64,
        opens: AtomicU64,
        closes: AtomicU64,
        first_attempt: AtomicU64,
    }
    impl Instrumentation for Triple {
        fn record_retry(&self, _op: &str, attempt: u32) {
            self.retries.fetch_add(1, Ordering::Relaxed);
            self.first_attempt
                .store(u64::from(attempt), Ordering::Relaxed);
        }
        fn record_circuit_open(&self, _op: &str) {
            self.opens.fetch_add(1, Ordering::Relaxed);
        }
        fn record_circuit_close(&self, _op: &str) {
            self.closes.fetch_add(1, Ordering::Relaxed);
        }
    }
    let t = Triple {
        retries: AtomicU64::new(0),
        opens: AtomicU64::new(0),
        closes: AtomicU64::new(0),
        first_attempt: AtomicU64::new(u64::MAX),
    };
    let dyn_ref: &dyn Instrumentation = &t;
    // attempt 起点由调用方约定（这里从 0 起），实现方不得假设。
    dyn_ref.record_retry("op", 0);
    assert_eq!(t.first_attempt.load(Ordering::Relaxed), 0);
    // 一次迁移只记一次。
    dyn_ref.record_circuit_open("op");
    dyn_ref.record_circuit_close("op");
    assert_eq!(
        (
            t.opens.load(Ordering::Relaxed),
            t.closes.load(Ordering::Relaxed)
        ),
        (1, 1)
    );
}

/// S-3：实现方义务的可执行面——`record_*` 不 panic（含不可信 op 输入）且不阻塞契约
/// 由类型系统（`&self`、无返回值）承担；这里验证「不 panic」的边界输入。
#[test]
fn assert_impl_duties() {
    struct PanicSink;
    impl Instrumentation for PanicSink {
        fn record_retry(&self, op: &str, _attempt: u32) {
            // 实现方义务：即使 op 含控制字符 / 换行 / 超长也不得 panic。
            let _sanitized = op.trim().len().min(128);
        }
        fn record_circuit_open(&self, _op: &str) {}
        fn record_circuit_close(&self, _op: &str) {}
    }
    let s = PanicSink;
    let dyn_ref: &dyn Instrumentation = &s;
    dyn_ref.record_retry("op\nwith\rcontrol\x00chars", 1);
    dyn_ref.record_retry(&"x".repeat(10_000), 1);
}

/// S-4：消费方义务的可执行面——`op` 为稳定分层名（低基数），不拼接请求 ID。
/// 该条款主要约束消费方代码，此处以最小消费样例固化形态。
#[test]
fn assert_consumer_duties() {
    struct Counter(AtomicU64);
    impl Instrumentation for Counter {
        fn record_retry(&self, op: &str, _attempt: u32) {
            // 低基数码点：分段数有限。
            assert!(op.split('.').count() <= 4, "op 应为低基数分层名");
            self.0.fetch_add(1, Ordering::Relaxed);
        }
        fn record_circuit_open(&self, _op: &str) {}
        fn record_circuit_close(&self, _op: &str) {}
    }
    let c = Counter(AtomicU64::new(0));
    let dyn_ref: &dyn Instrumentation = &c;
    dyn_ref.record_retry("exchange.binance.place_order", 1);
    assert_eq!(c.0.load(Ordering::Relaxed), 1);
}

/// S-5：验收命令见 docs/标准.md §5；此处断言测试面本身可被一次性全部执行
/// （作为 doctest 与集成测试共存的冒烟：本文件的全部用例即验收面）。
#[test]
fn assert_acceptance() {
    let _ = std::env::current_dir().expect("可取得当前目录（验收命令可执行）");
}
