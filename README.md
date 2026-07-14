# Reprospan

**Replay a failed agent run, change one step, and prove the fix.**

**回放一次失败的 Agent 运行，修改一步，证明修好了。**

[![CI](https://github.com/ritaprieto900/reprospan/actions/workflows/ci.yml/badge.svg)](https://github.com/ritaprieto900/reprospan/actions/workflows/ci.yml)

Reprospan is a local-first debugger for tool-using AI agents. It turns a failed run into a redacted, portable reproduction bundle that can be replayed offline, patched at one step, compared semantically, and promoted into a CI regression test.

Reprospan 是一个本地优先的 AI Agent 调试工具。它把一次失败的运行冻结成脱敏、可移植的回放包，离线重放、修改一步、语义对比，然后推进 CI 做回归测试。

> Status: pre-alpha. ｜ 状态：pre-alpha。

---

## 产品边界 ｜ Product boundary

Reprospan is not another hosted LLM observability dashboard. The first release focuses on one workflow:

Reprospan 不是又一个托管的 LLM 可观测性面板。首发只聚焦一个工作流：

1. inspect a failed agent run ｜ 检视一次失败的 Agent 运行
2. freeze it into a safe replay bundle ｜ 冻结为安全的回放包
3. patch a recorded model or tool result ｜ 修改一个已记录的模型/工具结果
4. replay without external side effects ｜ 无外部副作用的回放
5. prove the change with deterministic evaluators ｜ 用确定性断言证明修好了

| 做 ｜ Does | 不做 ｜ Does NOT |
|---|---|
| 本地 loopback，零外部依赖 ｜ local loopback | hosted 遥测 / SaaS dashboard |
| metadata_only 默认 ｜ default capture policy | 采集 prompt / tool body / headers / env |
| 离线 simulated replay | live re-execution |
| patch/diff/eval 走 CLI/本地文件 | patch/diff/eval 暴露为 HTTP route |
| artifact digest 引用 + optional bytes | 强制存储 artifact 内容 |

---

## 开发 ｜ Development

Prerequisites ｜ 前置依赖：

- Node.js 24+
- pnpm 11.13+
- Rust 1.88 with rustfmt and clippy

```bash
pnpm install
pnpm check        # TypeScript 类型检查 + JSON Schema 验证
pnpm test         # 7 个测试
cargo test --workspace    # 17 个测试
cargo clippy --workspace --all-targets -- -D warnings
```

### 离线回放管道 ｜ Offline replay pipeline

`import → export → patch → diff → eval`：

```bash
cargo run -p reprospan-cli -- import \
  --db .reprospan/demo.sqlite \
  --bundle packages/contracts/fixtures/v1/failed-tool-run.bundle.json
cargo run -p reprospan-cli -- export \
  --db .reprospan/demo.sqlite \
  --bundle-id bundle_support_refund_001 > before.json
cargo run -p reprospan-cli -- patch \
  --bundle before.json \
  --patch packages/contracts/fixtures/v1/fix-tool-result.patch.json > after.json
cargo run -p reprospan-cli -- diff --before before.json --after after.json
cargo run -p reprospan-cli -- eval \
  --bundle after.json \
  --eval packages/contracts/fixtures/v1/fix-tool-result.eval.json
```

所有命令向 stdout 输出一个 JSON document，可以直接用 shell 重定向组合管道。eval 通过退出 0，失败断言仍输出完整 JSON 但退出 1。

Commands write one JSON document to stdout — shell redirection composes the pipeline. Passed eval exits `0`; failed assertions still output valid JSON but exit `1`.

### 一键回放 ｜ One-shot replay

```bash
cargo run -p reprospan-cli -- replay \
  --db .reprospan/replay.sqlite \
  --bundle packages/contracts/fixtures/v1/failed-tool-run.bundle.json \
  --patch packages/contracts/fixtures/v1/fix-tool-result.patch.json \
  --eval packages/contracts/fixtures/v1/fix-tool-result.eval.json
```

### 内建 Demo ｜ Built-in demo

```bash
cargo run -p reprospan-cli -- demo --db .reprospan/demo.sqlite
```

### Loopback API

```bash
cargo run -p reprospan-cli -- serve --db .reprospan/server.sqlite
```

v1 API 端点：

| 方法 | 路径 | 说明 |
|---|---|---|
| `GET` | `/healthz` | 健康检查 |
| `GET` | `/v1/bundles` | 列出所有已导入的 bundle |
| `POST` | `/v1/bundles/ingest` | 导入 canonical bundle |
| `GET` | `/v1/bundles/{id}/timeline` | 读取 bundle 事件时间线 |
| `PUT` | `/v1/artifacts/{sha256}` | 上传 artifact bytes |

---

## TypeScript SDK（provider-neutral 采集）

`@reprospan/sdk` 构建 metadata-only canonical bundle 并提交给 loopback API。
不调用模型 provider，也不接受 prompt body、tool payload、headers、环境变量或凭据。

```bash
cargo run -p reprospan-cli -- serve --db .reprospan/local-agent.sqlite
pnpm --filter @reprospan/example-local-agent-capture build
pnpm --filter @reprospan/example-local-agent-capture start
```

示例执行真实的本地工具调用，记录六事件失败运行，经 Rust 验证和 SQLite 路径写入。
`model.*` 事件描述本地决策步骤，不声称调用了真实模型。回放仍然是 recorded 且无副作用。

Provider adapters 可用：

| 包 | Provider |
|---|---|
| `@reprospan/adapter-openai` | OpenAI |
| `@reprospan/adapter-anthropic` | Anthropic |

---

## 项目结构 ｜ Project structure

```
reprospan/
├── crates/
│   ├── reprospan-core/      # Bundle/Event/Patch/Eval 类型 + 验证 + 语义 diff
│   ├── reprospan-store/     # SQLite 存储：bundle、event projection、artifact bytes
│   ├── reprospan-server/    # Axum loopback HTTP API
│   └── reprospan-cli/       # CLI：import/export/patch/diff/eval/demo/replay/serve
├── packages/
│   ├── contracts/           # JSON Schema v1 + AJV 校验 + fixtures
│   ├── sdk/                 # @reprospan/sdk：CaptureSession + LoopbackClient
│   └── adapters/
│       ├── openai/          # OpenAI → canonical event 安全投影
│       └── anthropic/       # Anthropic → canonical event 安全投影
├── examples/
│   ├── local-agent-capture/ # 纯本地 deterministic agent 示例
│   └── openai-agent/        # 真实 OpenAI API + SDK 采集示例
├── apps/
│   └── web/                 # Timeline 可视化 + Diff 对比面板
├── docs/architecture/       # 产品边界、隐私模型、bundle 格式、replay 语义
└── .github/workflows/       # CI：TS check+test / Rust fmt+clippy+test / replay 回归
```

---

## 数据流 ｜ Data flow

```
TypeScript                    Rust
───────────                   ─────
CaptureSession                Bundle::validate()
  ↓                             ↓
assertValidBundle (AJV)       Store::import_bundle()
  ↓                             ↓
LoopbackClient.ingest()  →    POST /v1/bundles/ingest
                                 ↓
                              SQLite (bundles + events + artifacts)
                                 ↓
                              export / timeline / replay
                                 ↓
                              patch → semantic_diff → evaluate
```

---

## License

Apache-2.0.
