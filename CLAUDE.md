# ApiCat — Tauri 2 + Vue 3 桌面应用

接口调试桌面工具。前端 Vue 3 + TypeScript + NaiveUI + Pinia,后端 Rust(sqlx + sqlite)。

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

## Gotchas

- 代码/JSON 编辑区字号(monospace)不随主题缩放,属刻意排除
- `.url-highlight-layer` 需与 n-input medium 内边距同步,改动会错位
- dev 进程后台跑时 apicat.exe 存活,vite HMR 实时生效
