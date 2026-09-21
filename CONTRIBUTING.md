# CONTRIBUTING.md — 贡献指南（instrumentationx）

本文件面向贡献者，汇总本地门禁与提交约定。
AI Agent 的工作约定另见 [`AGENTS.md`](./AGENTS.md)；术语与领域语言见 [`CONTEXT.md`](./CONTEXT.md)。

## 开发流程

- 本仓库是**独立的单 crate 仓库**，不依赖 `xhyper.rs` 主工程及其内部 crate（`kernel` / `contracts` 等），
  也没有任何 `path` 依赖（`[dependencies]` 与 `[dev-dependencies]` 均为空）。
- substantial 变更走 feature branch → PR → review → merge，**禁止直接 push `main`**。
- `main` 已启用分支保护：要求 PR + 必需检查 `fmt / clippy / test`，
  `required_approving_review_count = 0`（单人也能合并），禁止强推与删除。
- 合并方式固定为 **create a merge commit**。注意仓库设置是
  `merge_commit_title = MERGE_MESSAGE` + `merge_commit_message = PR_TITLE`，因此
  `gh pr merge` 必须显式传 `--subject` 与 `--body`，否则会产出通用
  `Merge pull request #N from …` 标题。
- 提交信息遵循 Conventional Commits（`feat:` / `fix:` / `docs:` / `ci:` / `chore:` / `refactor:`），
  描述用简体中文。

## 本地门禁（P0 三件套）

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

本仓库无 feature、无 `--all-features` 变体；基准（可选）：

```bash
cargo bench --bench hot_path
cargo run --release --bench hot_path -- --quick
```

元数据完整性门禁（**不发布 crates.io**，此命令只校验打包元数据）：

```bash
cargo package --no-verify --allow-dirty
```

本仓库无 `path` 依赖，因此不需要额外的 `--offline` / `--config patch` 覆盖。

## 复用口径（不发布 crates.io）

- 本 crate **不发布到 crates.io**，仅以 GitHub 源码 / git 依赖形式复用。
- 文档与元数据中不得出现「可独立发布」「可直接 `cargo publish`」等表述，
  也不得放置 crates.io / docs.rs 徽章与外链。
- `Cargo.toml` 的 `documentation` 指向 `https://github.com/bytechainx/instrumentationx#readme`。
- 消费方引入方式（README「安装」小节为准）：

  ```toml
  [dependencies]
  instrumentationx = { git = "https://github.com/bytechainx/instrumentationx" }
  ```

## 开发约定

- 注释、文档、错误消息使用**简体中文**；标识符保持英文。
- 本 crate **无错误类型**（trait 方法不返回 `Result`），禁止引入 `thiserror` / `anyhow`。
- **保持零依赖是本 crate 的核心契约**：`[dependencies]` / `[dev-dependencies]` 必须始终为空，
  新增任何依赖前必须先升级讨论。
- trait 必须保持**对象安全**（消费方以 `&dyn Instrumentation` 使用）；新增方法须评估对象安全性，
  且因会破坏所有既有实现方而属于破坏性变更。
- 不在库代码里裸 `unwrap()`（`[lints.clippy]` 已 `deny` `unwrap_used` / `expect_used` / `panic` /
  `unreachable` / `todo` / `unimplemented`）；`#![forbid(unsafe_code)]`。
- 所有 `pub` 项必须有中文 `///` 文档（`missing_docs` 已 `deny`）。
- MSRV `1.70`、edition `2021`。
- 集成测试**必须离线运行**，不触碰真实网络。

## 提交前自检清单

- [ ] `cargo fmt --all -- --check` 通过
- [ ] `cargo clippy --all-targets -- -D warnings` 通过
- [ ] `cargo test --all-targets` 通过
- [ ] `cargo package --no-verify --allow-dirty` 通过
- [ ] 新增 `pub` 项都有中文 `///` 文档
- [ ] 文档中无「可独立发布」/ crates.io / docs.rs 表述
- [ ] trait 仍保持对象安全，且未引入任何依赖
