#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! E2E（instrumentationx）：在**进程内**端到端执行 `Instrumentation` 的**全部**公开接口。
//!
//! 本仓是共享契约仓（单一对象安全 trait，零依赖，无外部真服务），因此 E2E 的
//! 「端到端」落点是**跨 crate 边界的完整消费路径**：crate 外实现 trait，
//! 再经泛型单态与 trait object+跨线程两条路径真实调用，而不是只做单点冒烟。
//!
//! 对齐对象是 `cargo +nightly public-api --simplified` 导出的完整公开面：
//! `fn` / `type` / `field` / `const` / `variant` 五类逐条登记在 [`E2E_MANIFEST`]，
//! 运行期由 `cover` 登记表核对「声明 = 实际执行」（缺一即失败）。
//!
//! **独立核对**：`scripts/verify-e2e-coverage.mjs` 会重新派生公开面与清单双向 diff，
//! 并用 `-C instrument-coverage` + `llvm-cov report --show-functions` 断言每条公开
//! 函数执行次数 > 0；本文件内的登记表只是**声明**，不是唯一证据。
//!
//! ```text
//! cd /home/workspace/bytechainx/infra/instrumentationx
//! cargo test --test e2e_instrumentation
//! node scripts/verify-e2e-coverage.mjs instrumentationx --no-coverage
//! ```

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use instrumentationx::Instrumentation;

/// 公开面清单：`(条目类别, 入口 id)`，由 `cargo +nightly public-api --simplified` 派生并冻结。
///
/// 类别取值域：`fn` / `type` / `field` / `const` / `variant`。
/// 该清单是运行时登记的**唯一事实源**——`cover::hit` 拒绝清单外的 id，收尾断言拒绝
/// 「声明了却没执行」的条目。清单本身的时效性由外部核对器与公开面 diff 保证。
const E2E_MANIFEST: &[(&str, &str)] = &[
    ("type", "Instrumentation"),
    ("fn", "Instrumentation::record_circuit_close"),
    ("fn", "Instrumentation::record_circuit_open"),
    ("fn", "Instrumentation::record_retry"),
];

mod cover {
    use std::collections::BTreeSet;
    use std::sync::{Mutex, OnceLock};

    static EXECUTED: OnceLock<Mutex<BTreeSet<(&'static str, &'static str)>>> = OnceLock::new();

    fn log() -> &'static Mutex<BTreeSet<(&'static str, &'static str)>> {
        EXECUTED.get_or_init(|| Mutex::new(BTreeSet::new()))
    }

    /// 登记一次真实执行。清单外的 `(类别, id)` 立即 panic，防止调用点与清单漂移。
    pub fn hit(kind: &'static str, id: &'static str) {
        assert!(
            super::E2E_MANIFEST
                .iter()
                .any(|(declared_kind, declared_id)| *declared_kind == kind && *declared_id == id),
            "登记了清单外的公开条目：{kind} {id}"
        );
        log().lock().expect("覆盖登记表锁中毒").insert((kind, id));
    }

    /// 已登记的执行集合（收尾断言用）。
    pub fn executed() -> BTreeSet<(&'static str, &'static str)> {
        log().lock().expect("覆盖登记表锁中毒").clone()
    }
}

/// 覆盖登记的简写入口（保持调用点可读）。
fn hit(kind: &'static str, id: &'static str) {
    cover::hit(kind, id);
}

/// 清单自身良构：类别取值域合法、`(类别, id)` 不重复。
fn assert_manifest_wellformed() {
    let mut seen: BTreeSet<(&str, &str)> = BTreeSet::new();
    for (kind, id) in E2E_MANIFEST {
        assert!(
            matches!(*kind, "fn" | "type" | "field" | "const" | "variant"),
            "未知条目类别 {kind}（id={id}）"
        );
        assert!(seen.insert((*kind, *id)), "清单重复条目：{kind} {id}");
    }
}

/// 覆盖完整性：清单里每一条都必须被真实执行过。
fn assert_coverage_complete() {
    let executed = cover::executed();
    let mut missing: Vec<(&str, &str)> = Vec::new();
    for (kind, id) in E2E_MANIFEST {
        if !executed.contains(&(*kind, *id)) {
            missing.push((kind, id));
        }
    }
    assert!(missing.is_empty(), "声明了却未执行：{missing:?}");
}

/// crate 外的实现：E2E 必须验证「消费方能自己实现该契约」这一端到端含义。
#[derive(Debug, Default)]
struct Counting {
    retries: AtomicU64,
    opens: AtomicU64,
    closes: AtomicU64,
    last_attempt: AtomicU64,
}

impl Instrumentation for Counting {
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

fn assert_send_sync<T: Send + Sync + ?Sized>() {}

/// 单一驱动用例：保证阶段顺序与覆盖断言在同一个进程内完成。
#[test]
fn e2e_instrumentation_all_public_api() {
    assert_manifest_wellformed();

    // —— 契约形状：trait 本身可作 trait object，且承担 Send + Sync 约定 ——
    hit("type", "Instrumentation");
    assert_send_sync::<dyn Instrumentation>();
    assert_send_sync::<Counting>();

    // —— 消费者路径 1：泛型单态（编译期分发），crate 外实现 + crate 外调用 ——
    fn drive_generic<I: Instrumentation>(instrumentation: &I) {
        instrumentation.record_retry("db.query", 7);
        instrumentation.record_circuit_open("db.query");
        instrumentation.record_circuit_close("db.query");
    }

    let generic = Counting::default();
    hit("fn", "Instrumentation::record_retry");
    hit("fn", "Instrumentation::record_circuit_open");
    hit("fn", "Instrumentation::record_circuit_close");
    drive_generic(&generic);
    assert_eq!(generic.retries.load(Ordering::Relaxed), 1);
    assert_eq!(generic.opens.load(Ordering::Relaxed), 1);
    assert_eq!(generic.closes.load(Ordering::Relaxed), 1);
    assert_eq!(generic.last_attempt.load(Ordering::Relaxed), 7);

    // —— 消费者路径 2：trait object 跨线程共享（Send + Sync 的端到端含义）——
    let shared: Arc<dyn Instrumentation> = Arc::new(Counting::default());
    let mut handles = Vec::new();
    for worker in 0..4u32 {
        let shared = Arc::clone(&shared);
        handles.push(std::thread::spawn(move || {
            shared.record_retry("worker.tick", worker + 1);
            shared.record_circuit_open("worker.tick");
            shared.record_circuit_close("worker.tick");
        }));
    }
    for handle in handles {
        handle.join().expect("消费者线程不得 panic");
    }

    assert_coverage_complete();
}
