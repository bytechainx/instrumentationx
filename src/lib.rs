//! instrumentationx —— 可观测性注入点契约。
//!
//! 本 crate 只定义 [`Instrumentation`] 这一个 trait，用来解耦「可观测性实现」
//! （例如 `observex`）与「可观测性消费方」（例如 `resiliencx`）：
//!
//! - 消费方只依赖本 crate 的 trait，不需要引入任何具体的观测实现；
//! - 实现方（例如 `observex::TracingInstrumentation`）只实现本 trait；
//! - 双方互不依赖，因此不会形成循环依赖或反向业务依赖。
//!
//! 本 crate **零依赖**，只含一个对象安全（object-safe）的同步 trait。

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(unreachable_pub)]

/// 可观测性注入点。
///
/// 实现方负责把三类事件落到自己的观测后端（`tracing`、metrics 或自定义有界 sink）；
/// 消费方（弹性、调度等基础设施）只在事件发生时调用，不关心落地方式。
///
/// # 约定
///
/// - `op` 由调用方提供，**可能包含不可信输入**。实现方在写入观测后端前**必须**
///   做清理与长度限制，避免控制字符或超长字符串污染日志。
/// - `attempt` 的起点（`0` 或 `1`）由调用方约定，实现方不得假设。
/// - 实现必须同时满足 `Send + Sync`。
/// - 三类方法都不应阻塞调用方：慢后端应由实现方自行做有界缓冲或异步落地。
///
/// # Examples
///
/// ```
/// use instrumentationx::Instrumentation;
///
/// #[derive(Debug, Default)]
/// struct Noop;
///
/// impl Instrumentation for Noop {
///     fn record_retry(&self, _op: &str, _attempt: u32) {}
///     fn record_circuit_open(&self, _op: &str) {}
///     fn record_circuit_close(&self, _op: &str) {}
/// }
///
/// // 对象安全：可以直接作为 trait object 在多处传递与共享。
/// let instrumentation: &dyn Instrumentation = &Noop;
/// instrumentation.record_retry("db.query", 1);
/// ```
pub trait Instrumentation: Send + Sync {
    /// 记录一次重试（`attempt` 从 1 起或由调用方约定）。
    fn record_retry(&self, op: &str, attempt: u32);
    /// 记录熔断打开。
    fn record_circuit_open(&self, op: &str);
    /// 记录熔断关闭。
    fn record_circuit_close(&self, op: &str);
}

#[cfg(test)]
mod tests {
    use super::Instrumentation;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[derive(Debug, Default)]
    struct Counter {
        retries: AtomicU64,
        opens: AtomicU64,
        closes: AtomicU64,
        last_attempt: AtomicU64,
    }

    impl Instrumentation for Counter {
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
    fn trait_is_object_safe_and_usable_through_dyn() {
        let counter = Counter::default();
        let instrumentation: &dyn Instrumentation = &counter;
        instrumentation.record_retry("op", 3);
        instrumentation.record_circuit_open("op");
        instrumentation.record_circuit_close("op");
        assert_eq!(counter.retries.load(Ordering::Relaxed), 1);
        assert_eq!(counter.opens.load(Ordering::Relaxed), 1);
        assert_eq!(counter.closes.load(Ordering::Relaxed), 1);
        assert_eq!(counter.last_attempt.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn trait_object_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync + ?Sized>() {}
        assert_send_sync::<dyn Instrumentation>();
    }

    #[test]
    fn implementations_are_shareable_across_threads() {
        use std::sync::Arc;
        let counter = Arc::new(Counter::default());
        let instrumentation: Arc<dyn Instrumentation> = counter.clone();
        let worker = std::thread::spawn(move || {
            instrumentation.record_retry("worker.op", 1);
        });
        worker.join().expect("工作线程不应 panic");
        assert_eq!(counter.retries.load(Ordering::Relaxed), 1);
    }
}
