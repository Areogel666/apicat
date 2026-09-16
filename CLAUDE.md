# ApiCat — Tauri 2 + Vue 3 桌面应用

接口调试桌面工具。前端 Vue 3 + TypeScript + NaiveUI + Pinia,后端 Rust(sqlx + sqlite)。

> 目录结构地图 / IPC 约定 / 命名与反模式清单见 `AGENTS.md`(根目录及 `src/stores/`、`src/components/layout/`、`src-tauri/src/` 等子目录各有分册);本文档只放 Claude 动手时需要的东西。

## Commands

```bash
npm install              # 安装前端依赖
npm run tauri dev        # 开发模式(vite :1420 + cargo debug + 桌面窗口,HMR 热更新)
npx vue-tsc --noEmit     # 前端类型检查
npm run build            # 前端产物构建(vue-tsc + vite build)
npm run tauri build      # 打包 release 安装包(Windows 出 .msi)
```

## 主题系统(themeStore)

- `src/stores/theme.ts`:setup store。`customTokens` 增量覆盖 + `density`/`radiusScale`/`fontSize` → `resolvedTokens` computed → `applyTheme()` 写 `<html data-theme/data-density/data-font-size>` + inline style。
- `src/styles/tokens.css`:风格变量 `--row-height`/`--input-height`/`--spacing-*`/`--font-size-*`/`--radius-*`,按 `[data-density]`/`[data-font-size]` 选择器作用到 html。
- **组件样式必须用 CSS 变量而非硬编码 px**,否则不随主题风格档位变化。
- **Gotcha**:Pinia setup store 顶层解构 computed 是一次性快照非响应式,需用 `computed(() => store.xxx)`。
- 回退机制:打开主题工作室 `snapshotTheme()` 缓存 → 编辑实时改 DOM → 关闭弹窗 `applyCustomTheme()` 写盘;`revertTheme()` 恢复到打开前。**已去掉「应用主题」按钮**。

## 发布流程

- 版本号三处必须同步:package.json / src-tauri/tauri.conf.json / src-tauri/Cargo.toml
- git tag 格式 `v*.*.*`(如 v1.0.3),触发 `.github/workflows/release.yml` 构建;`releaseDraft: true` → 需手动 Publish
- **推 tag 不会自动推分支**:`git push origin master v1.0.3` 分开推

## 核心语义（1.0.4 起，改动前先读）

- **数据字典 = 「字段名绑定」**：项目级 `field_dictionary_rules`（字段名↔字典，一对一）+ 接口级 `field_dictionary_overrides`（例外，优先）。描述列未绑定 → 直接 `n-input`；已绑定 → `FieldDictDesc.vue`（命中项 + Tooltip 全枚举 + 手写描述尾部）+ ✎ 手写编辑 popover + ✕ 解绑；绑定入口 📖 弹窗支持「项目规则 / 仅当前接口」双作用域（再点已绑定字典 = 取消）。Tooltip 枚举内容块自带 `--bg-elevated` 背景防对比问题。
- **参数元数据（type/description）随接口 `params` JSON 持久化**：`parseKvText/parseJsonToParams/parseTextToParams` 均带 `prev` 同 key 合并；用例快照缺元数据时由接口定义补全（`mergeParamMeta`）。**已作废旧 `descriptionDictRef` 字段，不再流动**（类型保留兼容旧数据）。
- **草稿隔离**：切换接口保存/恢复草稿时参数数组一律深拷贝；落库只发生在切走瞬间并锁定离开的接口 id（`flushPersist`），不在编辑期做防抖写库。
- **URL-Encoded 的 body 存储 = 结构化 JSON 数组**（含 type/description/enabled），旧 `k=v` 文本加载用 `parseUrlencodedBody` 兼容；发送时才 `syncUrlencodedData()` 编码为 k=v。保存路径统一用 `urlencodedStorageBody()`，勿直接存 bodyContent。
- **布局常驻**：`AppLayout` 中接口树/字典树侧栏与 `MainPanel`/`DictionaryJsonPanel` 都用 `v-show` 常驻（切换保留浏览状态），不要改回 `v-if` 卸载重挂。

## Gotchas

- 代码/JSON 编辑区字号(monospace)不随主题缩放,属刻意排除
- `.url-highlight-layer` 需与 n-input medium 内边距同步,改动会错位
- dev 进程后台跑时 apicat.exe 存活,vite HMR 实时生效
