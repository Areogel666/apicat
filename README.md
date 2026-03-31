<div align="center">
  <h1>🐱 ApiCat</h1>
  <p>
    A lightweight, high-performance API debugging and testing tool built with Tauri, Rust, and Vue 3.<br/>
    一款基于 Tauri、Rust 和 Vue 3 构建的轻量、高性能 API 调试与测试工具。
  </p>
  <p>
    <img src="https://img.shields.io/badge/version-1.0.0--beta-orange" alt="version"/>
    <img src="https://img.shields.io/badge/platform-Windows-blue" alt="platform"/>
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
- **Productivity Boosters**: Inline method selector with color coding, URL ↔ QueryParams two-way binding, cURL import/export, drag-and-drop sorting, syntax-highlighted response viewer.
- **Unsaved Change Indicator**: Orange dot on request nodes signals unsaved modifications; green dot confirms a successful save (auto-fades after 1.5s).

### 🚀 Quick Start (For Users)

1. Download the latest installer (`.msi` or `.exe`) from the [Releases](../../releases) page.
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

See [docs/release/1.0.0-beta.md](docs/release/1.0.0-beta.md) for the full release notes.

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
- **效率工具**：Method 内联彩色选择器、URL ↔ Params 双向绑定、cURL 导入导出、拖拽排序、JSON 高亮与折叠。
- **暂存状态指示**：左侧树节点旁的橙色圆点表示有未保存修改，绿色圆点表示刚保存成功（1.5s 后自动消失）。

### 🚀 快速使用

1. 从 [Releases](../../releases) 页面下载最新安装包（`.msi` 或 `.exe`）。
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
| 左侧边栏 | 接口树，支持文件夹嵌套、拖拽排序、搜索过滤；悬停节点显示 `+` / `•••` 操作按钮 |
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

完整的 Beta 版修复记录详见 [docs/release/1.0.0-beta.md](docs/release/1.0.0-beta.md)。

### 📄 开源协议

本项目采用 [MIT License](LICENSE) 开源协议。
