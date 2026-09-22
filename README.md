<div align="center">
  <h1>🐱 ApiCat</h1>
  <p>
    A lightweight, high-performance API debugging and testing tool built with Tauri, Rust, and Vue 3.<br/>
    一款基于 Tauri、Rust 和 Vue 3 构建的轻量、高性能 API 调试与测试工具。
  </p>
  <p>
    <img src="https://img.shields.io/badge/version-1.0.5-orange" alt="version"/>
    <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue" alt="platform"/>
    <img src="https://img.shields.io/badge/license-MIT-green" alt="license"/>
  </p>
</div>

<p align="center">
  <a href="#english">English</a> •
  <a href="#中文">中文</a>
</p>

---

<h2 id="english">🇬🇧 English</h2>

ApiCat is a modern desktop application for API debugging and testing, powered by Tauri 2.0, Rust, and Vue 3. It eliminates CORS restrictions, offers a fast and native experience, and comes with a built-in stress testing engine.

### ✨ Features

- **No CORS Restrictions**: All HTTP requests are dispatched natively via Rust `reqwest`, bypassing browser CORS entirely.
- **Advanced Auth Support**: One-click configuration for Bearer Token, Basic Auth, and API Keys.
- **Environment & Variables**: Multi-environment management with `{{variable}}` syntax in URLs, Headers, and Bodies.
- **Stress Testing**: Built-in concurrent stress testing engine powered by Rust's `tokio`. Supports test-case parameterization.
- **Data Portability**: Import/Export support for Postman Collections (v2.1) and OpenAPI (Swagger) specifications.
- **Draft Cache**: Unsaved edits are automatically cached when switching requests and restored on return — no changes lost.
- **Request History**: Automatic request history recording with side-by-side Diff comparison between responses.
- **Public Header Templates**: Save and reuse common headers (e.g., Auth tokens) across requests.
- **Multi-Tab Workflow**: Open multiple requests in parallel tabs, switch freely, and restore session state across restarts.
- **Productivity Boosters**: Inline method selector with color coding, URL ↔ QueryParams two-way binding, cURL import/export, drag-and-drop sorting, syntax-highlighted response viewer.
- **Unsaved Change Indicator**: Orange dot on request nodes signals unsaved modifications; green dot confirms a successful save (auto-fades after 1.5s).
- **Theme Studio**: Customize colors, layout density, corner radius, and font size with live preview — preset themes, contrast checking, and theme JSON import/export.
- **Data Dictionary Field Binding**: Bind a dictionary to a field name (project-level rules + per-request overrides); every same-named field across requests auto-resolves its value to a meaning, with hover to view the full enum. Copy a dictionary (with its rules) to another project.
- **Param Metadata**: Field `type` / `description` persisted per request — survive switching requests, test cases, and KV/JSON/Text modes; never bleed across requests.
- **Stress History Tab**: A dedicated "Stress" tab beside the request editor listing past runs with report download (Markdown) and two-run comparison.
- **Localhost HTTP Bridge**: A local REST service on `127.0.0.1:17320` (token-secured) that lets AI skills read and write projects, requests, dictionaries, test cases, and stress runs — data stays in one place instead of scripts touching the database directly.
- **AI Skills Installer**: One-click install/uninstall of four bundled skills — `apicat-lib` (Bridge access base), `apicat-edit` (edit requests/dictionaries, batch-import from Markdown docs), `apicat-test-gen` (generate test cases / stress runs), `apicat-doc-gen` (generate API docs) — into your Agent's skills directory via junction links; dangling links are auto-repaired on app startup.
- **Test Assertions**: Test cases can declare assertions (`status_code` / JSON Path); running a case gives a clear pass/fail verdict.
- **Stress Report Preview & Thresholds**: Built-in Markdown report generator shared by the app and the Bridge, with in-app preview and Markdown/HTML export. P95/P99 reference lines are configurable per project with per-request overrides.
- **API Docs Panel**: A third sidebar tab browsing your local API documentation directory — open files directly or reveal them in the file explorer.
- **Response Dictionary Translation**: A hand-rolled JSON tree renders responses with color-coded micro-labels next to values that hit a bound dictionary item (`value ▸ meaning`, hover for dict name + description) — read-only translation, no schema changes.
- **Request Editor Undo/Redo**: Structural undo/redo (add/remove rows, sorting, clear, paste, URL/method) with a per-request stack (depth 20); edit area shows a "⚠ unsaved" indicator with one-click restore to the last Ctrl+S version.
- **History Merged & Inline Preview**: `test_case_history` merged into `request_history`; click a history row for inline response preview (aligned with live rendering); large response bodies stored as files with save-as.
- **Dictionary Management Rework**: Two-tab management page; sidebar rows show dict code/name on two lines with item description on hover; binding dropdowns sorted by code with search.
- **Collapsible Sidebar**: A consistent collapse button across all three sidebar tabs — collapse to full-width content, state persisted.
- **Docs Markdown Preview**: In-app Markdown preview for doc files (scrolling fixed), plus a defensive placeholder for unsupported file types.

### 🚀 Quick Start (For Users)

1. Download the latest installer (`.msi` or `.exe`) from the [Releases](https://github.com/Areogel666/apicat/releases) page.
2. Install and launch **ApiCat**.
3. Click **New Project** in the top bar to create a project.
4. Use the `+` button in the sidebar to create a Collection (folder) and Request.
5. Enter your API endpoint URL, select the HTTP method, and configure parameters/authentication.
6. Hit **Send** and view the formatted response in the bottom panel.
7. Press **Ctrl+S** to save the request at any time.

### 🛠️ Development (For Developers)

Ensure you have **Node.js** (v20+), **Rust** (`rustup`, `cargo`), and **C++ Build Tools** (e.g., Visual Studio 2022) installed.

```bash
# Clone and enter the repository
cd apicat

# Install frontend dependencies
npm install

# Start the development server (Vite + Tauri hot reload)
npm run tauri dev

# Build the production executable and installer
npm run tauri build
```

> **Windows tip**: If you encounter Rust compilation errors, run from a  
> *"x64 Native Tools Command Prompt for VS 2022"* terminal.

Build output: `src-tauri/target/release/bundle/msi/`

### 📋 Changelog

- Latest stable: [docs/release/1.0.5.md](docs/release/1.0.5.md)
- Previous: [docs/release/1.0.4.md](docs/release/1.0.4.md)
- Beta history: [docs/release/1.0.0-beta.md](docs/release/1.0.0-beta.md)

### 📄 License

This project is licensed under the [MIT License](LICENSE).

---

<h2 id="中文">🇨🇳 中文</h2>

ApiCat 是一款基于 Tauri 2.0 + Rust + Vue 3 的轻量、高性能、无跨域限制的 API 桌面调试与测试工具。

### ✨ 核心特性

- **无跨域限制**：所有 HTTP 请求均由 Rust 底层（`reqwest`）原生发出，彻底绕过浏览器 CORS 限制。
- **多种认证方式**：支持一键配置 Bearer Token、Basic Auth 和 API Key。
- **环境与全局变量**：多环境管理，支持在 URL、Header 或 Body 中通过 `{{变量名}}` 语法注入变量。
- **原生并发压测**：基于 Rust `tokio` 异步运行时的压测引擎，防 OOM，支持绑定测试用例参数化执行。
- **数据迁移**：支持导入/导出 Postman Collection v2.1，以及导入 OpenAPI (Swagger) 文档。
- **草稿缓存**：切换接口时自动暂存未保存的编辑，切回时自动恢复，修改不丢失。
- **请求历史**：自动记录每次请求历史，支持 Diff 对比两次响应差异。
- **公共 Headers 模板**：将 Token、Content-Type 等常用 Header 保存为模板，一键应用。
- **多标签页工作流**：并行打开多个请求标签页，自由切换，重启后恢复工作现场。
- **效率工具**：Method 内联彩色选择器、URL ↔ Params 双向绑定、cURL 导入导出、拖拽排序、JSON 高亮与折叠。
- **暂存状态指示**：左侧树节点旁的橙色圆点表示有未保存修改，绿色圆点表示刚保存成功（1.5s 后自动消失）。
- **主题工作室**：自由定制配色、布局密度、圆角与字号，实时预览；内置预设主题、对比度检查与主题 JSON 导入导出。
- **数据字典「字段名绑定」**：项目级「字段名 ↔ 字典」规则 + 接口级例外；项目内所有同名字段按当前取值自动命中展示含义，悬停查看全部枚举；可把字典（含规则）整体复制到其它项目。
- **参数元数据持久化**：字段「类型 / 描述」随每个接口独立保存，切换接口、用例或 KV/JSON/Text 模式都不丢、不串扰。
- **压测历史 Tab**：编辑区新增常驻「压测」页，本接口历史回看、Markdown 报告下载、双条历史对比。
- **本地 HTTP Bridge**：`127.0.0.1:17320` 上的本地 REST 服务（token 鉴权），供 AI 技能读写项目、接口、字典、用例与压测数据——数据真源收敛到一处，脚本不再直连数据库。
- **AI 技能安装器**：一键把 4 个内置技能安装/卸载到各 Agent 的 skills 目录（junction 链接）：`apicat-lib`（Bridge 访问底座）、`apicat-edit`（增删改接口/字典、从 Markdown 文档批量导入）、`apicat-test-gen`（生成用例/压测）、`apicat-doc-gen`（生成接口文档）；App 启动时自动修复悬空链接。
- **用例断言**：测试用例可声明断言（`status_code` / JSON Path），运行后给出明确的通过/失败判定。
- **压测报告预览与参考线**：Markdown 报告生成器在 Rust 侧下沉，App 与 Bridge 共用同一真源；支持应用内预览与 Markdown/HTML 导出；P95/P99 参考线可按项目配置、按接口覆盖。
- **接口文档面板**：侧边栏第三个 Tab，浏览本地接口文档目录，双击打开文件或在文件管理器中定位。
- **响应字段字典翻译**：自研 JSON 树渲染响应，字段值命中已绑定字典项时值旁显示 `value ▸ 含义` 字典色微标签（hover 看字典名 + 描述）——只读翻译，不加数据模型。
- **接口编辑撤销/重做**：结构级 undo/redo（加删行、排序、清空、粘贴、URL/method），按接口分栈（深 20）；编辑区「⚠ 未保存」标记 + 一键回到上次 Ctrl+S 版本。
- **历史记录增强**：`test_case_history` 并入 `request_history`；点击历史行行内预览响应（对齐实时渲染）；大响应体文件化存储 + 另存为。
- **字典管理改版**：管理页双 Tab；左侧栏两行化（code 主 / name 弱化）+ 字典项描述悬停；绑定下拉按 code 排序 + 可搜索。
- **左侧栏折叠**：接口/字典/文档三栏统一折叠按钮，折叠后编辑区全宽，状态持久化。
- **文档 Markdown 预览**：文档面板内直接预览 Markdown（含滚动修复）；非 md 类型给出防御占位。

### 🚀 快速使用

1. 从 [Releases](https://github.com/Areogel666/apicat/releases) 页面下载最新安装包（`.msi` 或 `.exe`）。
2. 双击安装并启动 **ApiCat**。
3. 点击顶部「新建项目」创建一个项目。
4. 在左侧边栏使用 `+` 按钮新建 Collection（文件夹）和 Request（接口）。
5. 在顶部输入 URL，选择 HTTP 方法，配置参数和认证信息。
6. 点击 **Send（发送）** 按钮，在底部面板查看响应结果。
7. 随时按 **Ctrl+S** 保存当前接口。

### 🖥️ 界面说明

| 区域 | 说明 |
|------|------|
| 顶部栏 | 项目切换 / 环境选择 / 设置菜单（导入导出、Header 模板等）|
| 左侧边栏 | 三个 Tab：接口树 / 字典树 / 接口文档；接口树支持文件夹嵌套、拖拽排序、搜索过滤，悬停节点显示 `+` / `•••` 操作按钮 |
| 中央主面板 | URL 栏（内嵌 Method 选择器 + 环境标签）+ Params / Headers / Body / Auth 配置 |
| 底部响应面板 | 响应状态 / Body（支持 JSON 折叠、XML/HTML 高亮）/ Headers / Cookie / 历史记录 / 压测结果 |

### 🛠️ 本地开发与编译

如需修改源码或自行编译，请确保 Windows 环境已安装：

- **Node.js** (推荐 v20+) & **npm**
- **Rust** 工具链 (`rustup`, `cargo`)
- **C++ 编译工具**（推荐安装 Visual Studio 2022 Community，勾选"使用 C++ 的桌面开发"）

```bash
# 进入项目目录
cd apicat

# 安装前端依赖
npm install

# 启动开发服务器（支持 Vite 热更新与 Tauri 后端）
# 提示：Windows 下若遇到编译错误，请在 "x64 Native Tools Command Prompt for VS 2022" 中运行
npm run tauri dev

# 构建生产环境可执行文件及安装包
npm run tauri build
```

构建完成后，安装包位于 `src-tauri/target/release/bundle/msi/` 目录。

### 📋 更新记录

- 最新稳定版说明： [docs/release/1.0.5.md](docs/release/1.0.5.md)
- 上一版本说明： [docs/release/1.0.4.md](docs/release/1.0.4.md)
- Beta 阶段历史记录： [docs/release/1.0.0-beta.md](docs/release/1.0.0-beta.md)

### 📄 开源协议

本项目采用 [MIT License](LICENSE) 开源协议。
