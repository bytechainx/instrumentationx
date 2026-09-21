# instrumentationx

`instrumentationx` 是一个**零依赖**的可观测性注入点契约 crate，只定义一个对象安全的
同步 trait：`Instrumentation`。

它解决的是「可观测性实现」与「可观测性消费方」之间的解耦问题：

```
        ┌──────────────────┐
        │ instrumentationx │  ← 只定义 trait
        └────────┬─────────┘
                 │
     ┌───────────┴───────────┐
     ▼                       ▼
  observex                resiliencx
（实现 trait）          （消费 trait）
```

双方都只依赖本 crate，不必互相依赖，因此不会形成循环依赖或反向业务依赖。

## 安装

本 crate **不发布到 crates.io**，通过 git 依赖引入：

```toml
[dependencies]
instrumentationx = { git = "https://github.com/bytechainx/instrumentationx" }
```

## 最小可运行示例

### 消费方：只依赖 trait

```rust
use instrumentationx::Instrumentation;

/// 调用方只需要一个 `&dyn Instrumentation`，不关心落地实现。
fn call_with_observed_retry(instrumentation: &dyn Instrumentation) {
    for attempt in 1..=3 {
        instrumentation.record_retry("db.query", attempt);
    }
}
```

### 实现方：只实现 trait

```rust
use instrumentationx::Instrumentation;

/// 落地到 `tracing`。注意：`op` 可能含不可信输入，写日志前必须清理并限长。
#[derive(Debug, Default)]
struct TracingInstrumentation;

impl Instrumentation for TracingInstrumentation {
    fn record_retry(&self, op: &str, attempt: u32) {
        tracing::info!(op, attempt, "retry");
    }
    fn record_circuit_open(&self, op: &str) {
        tracing::info!(op, "circuit_open");
    }
    fn record_circuit_close(&self, op: &str) {
        tracing::info!(op, "circuit_close");
    }
}

fn main() {
    let instrumentation = TracingInstrumentation;
    call_with_observed_retry(&instrumentation);
}
```

一个**零依赖、可编译**的完整示例（无需 `tracing`）：

```rust
use instrumentationx::Instrumentation;

#[derive(Debug, Default)]
struct Noop;

impl Instrumentation for Noop {
    fn record_retry(&self, _op: &str, _attempt: u32) {}
    fn record_circuit_open(&self, _op: &str) {}
    fn record_circuit_close(&self, _op: &str) {}
}

fn call_with_observed_retry(instrumentation: &dyn Instrumentation) {
    for attempt in 1..=3 {
        instrumentation.record_retry("db.query", attempt);
    }
}

fn main() {
    call_with_observed_retry(&Noop);
}
```

## trait 契约

| 方法 | 语义 |
| --- | --- |
| `record_retry(&self, op: &str, attempt: u32)` | 记录一次重试；`attempt` 起点由调用方约定 |
| `record_circuit_open(&self, op: &str)` | 记录熔断打开 |
| `record_circuit_close(&self, op: &str)` | 记录熔断关闭 |

约定：

- 实现必须满足 `Send + Sync`，可跨线程共享；
- 三类方法**不应阻塞**调用方：慢后端由实现方自行做有界缓冲或异步落地；
- `op` 由调用方提供、**可能包含不可信输入**，实现方写后端前必须清理与限长；
- trait 对象安全，可直接以 `&dyn Instrumentation` / `Arc<dyn Instrumentation>` 注入。

## 非目标

- 不是 OpenTelemetry API/SDK，不实现 OTLP；
- 不提供任何具体后端实现（`tracing` / metrics / sink 由 `observex` 等实现方提供）；
- 不定义异步观测接口（当前只需要同步记录点）。

## 许可

MIT OR Apache-2.0
