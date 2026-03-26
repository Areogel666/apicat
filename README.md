<div align="center">
  <h1>🐱 ApiCat</h1>
  <p>
    A lightweight, high-performance API debugging and testing tool built with Tauri, Rust, and Vue 3.<br/>
    一款基于 Tauri、Rust 和 Vue 3 构建的轻量、高性能 API 调试与测试工具。
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

- **No CORS Restrictions**: All HTTP requests are dispatched natively via Rust `reqwest`.
- **Advanced Auth Support**: One-click generation for Bearer Token, Basic Auth, and API Keys.
- **Environment & Variables**: Robust environment variables configuration with `{{variable}}` syntax support in URLs, Headers, and Bodies.
- **Stress Testing**: Built-in concurrent stress testing engine powered by Rust's `tokio` multi-task model.
- **Data Portability**: Import/Export support for Postman Collections and OpenAPI (Swagger) specifications.
- **Public Header Templates**: Save and reuse common headers across requests.
- **Productivity Boosters**: cURL command generation, drag-and-drop sorting, and syntax-highlighted response viewer.

### 🚀 Quick Start (For Users)

1. Download the latest installer (`.msi` or `.exe`) from the Releases page.
2. Install and launch **ApiCat**.
3. Click the `+` button in the sidebar to create a new Collection and Request.
4. Configure your API endpoint, parameters, and authentication.
5. Hit **Send** and view the formatted response.

### 🛠️ Development (For Developers)

To build ApiCat from source, ensure you have **Node.js** (v20+), **Rust** (`rustup`, `cargo`), and **C++ Build Tools** (e.g., Visual Studio 2022) installed.

```bash
# Clone the repository
cd apicat

# Install frontend dependencies
npm install

# Start the development server (Vite + Tauri)
npm run tauri dev

# Build the production executable and installer
npm run tauri build
```

### 📄 License

This project is licensed under the [MIT License](LICENSE).

---

<h2 id="中文">🇨🇳 中文</h2>

ApiCat 是一款基于 Tauri 2.0 + Rust + Vue 3 的轻量、高性能、无跨域限制的 API 桌面调试与测试工具。

### ✨ 核心特性

- **无跨域限制**：所有 HTTP 请求均在 Rust 底层（通过 `reqwest`）发送。
- **便捷认证**：支持一键设置 Bearer Token、Basic Auth 和 API Key。
- **环境与全局变量**：强大的环境变量管理，支持在 URL、Header 或 Body 中通过 `{{变量名}}` 调用。
- **并发压测**：基于 Rust `tokio` 异步运行时的原生并发压测引擎，防 OOM。
- **数据迁移**：支持导入导出 Postman Collection v2.1 及导入 OpenAPI (Swagger) 文档。
- **公共 Headers 模板**：将常用的 Token 或 Content-Type 保存为模板，一键应用到请求。
- **效率提升**：右键快速复制为 cURL、拖拽排序、JSON 响应高亮与折叠。

### 🚀 快速使用（普通使用者）

1. 从 Releases 页面获取安装包（`.msi` 或 `.exe` 文件）。
2. 双击运行并安装 **ApiCat**。
3. 在左侧边栏点击 `+` 按钮，新建 Collection（文件夹）和 Request（接口）。
4. 在顶部输入 URL，配置 HTTP 方法、参数和认证信息。
5. 点击 **发送 (Send)** 按钮，在底部面板查看格式化后的响应结果。

### 🛠️ 本地开发与编译（深度用户）

如果你需要修改源码或自行编译打包，请确保你的 Windows 环境已安装以下依赖：
- **Node.js** (推荐 v20+) & **npm**
- **Rust** 工具链 (`rustup`, `cargo`)
- **C++ 编译工具**（推荐安装 Visual Studio 2022 Community，勾选“使用 C++ 的桌面开发”）

```bash
# 进入项目目录
cd apicat

# 安装前端依赖
npm install

# 启动开发服务器（支持 Vite 热更新与 Tauri 后端）
# 提示：Windows 下若遇到编译错误，请在 "x64 Native Tools Command Prompt for VS 2022" 终端运行
npm run tauri dev

# 构建生产环境可执行文件及安装包
npm run tauri build
```
构建完成后，可在 `src-tauri/target/release/bundle/msi/` 目录下找到生成的安装包。

### 📄 开源协议

本项目采用 [MIT License](LICENSE) 开源协议。
