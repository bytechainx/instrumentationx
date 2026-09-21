# instrumentationx Agent 指南

> 本文件为 AI Agent 在本仓库工作时的入口指南。

## 项目定位

可观测性注入点契约：单一对象安全（object-safe）的同步 trait `Instrumentation`，用于解耦观测实现（如 `observex`）与观测消费方（如 `resiliencx`），双方互不依赖。

## 技术栈

- Rust edition 2021, rust-version 1.70
- **零依赖**：`[dependencies]` 与 `[dev-dependencies]` 均为空，不引入任何第三方 crate
- 零内部耦合：不依赖组织内其他 crate

## 代码结构

```text
src/
└── lib.rs          # 唯一模块：Instrumentation trait 定义（record_retry / record_circuit_open / record_circuit_close）
tests/
└── public_api.rs   # 公共 API 契约测试（对象安全、trait 方法签名）
benches/
└── hot_path.rs     # 热路径基准（harness = false，支持 --quick）
docs/
└── API.md          # API 文档
```

## 开发约定

- 注释与文档使用简体中文；标识符保持英文
- 本 crate 无错误类型（零依赖、trait 方法不返回 Result），禁止引入 thiserror/anyhow
- 禁止裸 `unwrap()`（库代码；crate 级 `[lints.clippy]` 已 deny `unwrap_used` / `expect_used` / `panic` / `unreachable` / `todo` / `unimplemented`）
- `#![forbid(unsafe_code)]`
- `#![deny(missing_docs)]`、`#![deny(unreachable_pub)]`
- trait 必须保持对象安全（消费方以 `&dyn Instrumentation` 使用），新增方法须评估对象安全性
- 保持零依赖是本 crate 的核心契约，新增任何依赖前必须升级讨论

## 门禁三件套（P0）

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

基准（可选）：

```bash
cargo bench --bench hot_path          # 完整
cargo run --release --bench hot_path -- --quick   # 快速
```

## 相关文档

- 组织 Rust 规范：`~/org-config/rulesets/rust/RULES.md`
- API 文档：`docs/API.md`
- 术语与领域语言：`CONTEXT.md`
- 贡献指南：`CONTRIBUTING.md`
- 变更记录：`CHANGELOG.md`
- 基准测试：`benches/hot_path.rs`
