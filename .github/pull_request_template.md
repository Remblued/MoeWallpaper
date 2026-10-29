## 这个 PR 做了什么

<!-- 一句话。写"加了缩略图网格"或"修复了路径拼接"，别写"更新代码" -->

## 改了谁的领地

- [ ] 改了 `src-tauri/**` 或 `sources/**`（A 的领地）
- [ ] 改了 `src/**` 或 `index.html`（B 的领地）

> 两个都勾 = 这个 PR 会打架。拆成两个。

## 合并前的检查

### 动过 Rust（`src-tauri/**`）

- [ ] `cargo fmt --check` 无输出
- [ ] `cargo clippy --all-targets -- -D warnings` 无警告
- [ ] `cargo test` 全绿，且测试数没有变少
- [ ] 新增的函数有对应的测试

### 动过前端（`src/**`、`index.html`）

- [ ] `npm run build` 通过（这一步会做 TypeScript 类型检查）
- [ ] `npm run tauri dev` 窗口能正常打开
- [ ] 窗口拉到最窄（800px）没有横向滚动条
- [ ] 加载中 / 空 / 出错三种状态都有可见文案，不是白屏

### 动过命令契约（`commands/mod.rs`、`ImageDto` 或 `sources/model.rs`）

- [ ] `docs/HANDOVER.md` §4 已同步
- [ ] 前端 `src/lib/api.ts` 的类型已同步
- [ ] **对方已经知情**——这类改动会打断对方正在做的事

## 截图

<!-- 动了界面的 PR 贴一张图。没有界面改动就写"没有界面改动" -->
