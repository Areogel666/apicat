// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// 仅 Linux:WebKitGTK 对亚像素抗锯齿(彩边重影)与 hinting 不足(粗体拉丁笔画
// 晕开)的渲染质量差,系统默认配置下选中态粗体文字发虚。生成一份进程内
// fontconfig 覆盖(灰度 AA + hintfull),只影响 ApiCat 自身,不动系统配置。
#[cfg(target_os = "linux")]
fn setup_linux_font_rendering() {
    const CONF: &str = r#"<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "fonts.dtd">
<fontconfig>
  <include ignore_missing="yes">/etc/fonts/fonts.conf</include>
  <match target="font">
    <edit name="rgba" mode="assign"><const>none</const></edit>
    <edit name="hinting" mode="assign"><bool>true</bool></edit>
    <edit name="hintstyle" mode="assign"><const>hintfull</const></edit>
  </match>
</fontconfig>"#;
    let path = std::env::temp_dir().join("apicat-fontconfig.conf");
    if std::fs::write(&path, CONF).is_ok() {
        std::env::set_var("FONTCONFIG_FILE", &path);
    }
}

// 仅 Linux：WebKitGTK 的 GPU 渲染路径在部分 Intel i915 驱动上会触发 GPU HANG
// （2026-11-10 实测：玻璃已关、无拖拽操作仍整机卡死，只有 ApiCat 运行时复现）。
// 启动时读 app-settings.json 的 linux.webkitRenderer，在 WebKit 初始化前设置环境变量：
//   auto     —— 保持 GPU 加速
//   shm      —— 禁用 DMABUF 渲染器（默认；回退共享内存传输，规避 hang）
//   software —— 再禁用加速合成（最稳，但整页 CPU 渲染）
// 外部显式设置的环境变量优先，方便临时覆盖排查。
/// Intel 核显（i915）是否在驱动本机显示。
/// 只有 i915 才存在已知的 GPU HANG 问题，AMD/NVIDIA 用户不应陪跑软件渲染。
#[cfg(target_os = "linux")]
fn gpu_is_intel_i915() -> bool {
    if !std::path::Path::new("/sys/module/i915").exists() {
        return false;
    }
    let Ok(entries) = std::fs::read_dir("/sys/class/drm") else { return true };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        // 只要 cardN，跳过 cardN-DP-1 这类连接器
        if !name.starts_with("card") || name.contains('-') {
            continue;
        }
        if let Ok(v) = std::fs::read_to_string(e.path().join("device/vendor")) {
            if v.trim().eq_ignore_ascii_case("0x8086") {
                return true;
            }
        }
    }
    // i915 已加载但读不到 vendor：保守按 Intel 处理
    true
}

#[cfg(target_os = "linux")]
fn default_linux_webkit_renderer() -> String {
    if gpu_is_intel_i915() { "software".into() } else { "auto".into() }
}

#[cfg(target_os = "linux")]
fn read_linux_webkit_renderer() -> String {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|h| std::path::PathBuf::from(h).join(".local").join("share"))
        });
    let Some(base) = base else { return default_linux_webkit_renderer() };
    let path = base.join("com.apicat.app").join("app-settings.json");
    let Ok(text) = std::fs::read_to_string(&path) else { return default_linux_webkit_renderer() };
    let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return default_linux_webkit_renderer();
    };

    // 用户显式设置过就尊重用户
    if let Some(v) = json.get("linux.webkitRenderer").and_then(|v| v.as_str()) {
        return v.to_string();
    }

    // 首次启动：按 GPU 探测，并写回配置，保证前端 UI 显示的档位与实际一致
    let mode = default_linux_webkit_renderer();
    json["linux.webkitRenderer"] = serde_json::Value::String(mode.clone());
    if let Ok(s) = serde_json::to_string_pretty(&json) {
        let _ = std::fs::write(&path, s);
    }
    mode
}

#[cfg(target_os = "linux")]
fn setup_linux_webview() {
    // 无障碍桥：AppImage 打包的 libatspi（22.04）与系统 at-spi2-core（24.04）协议
    // 不匹配，输入框获得焦点时会打印 "atk-bridge: get_device_events_reply: unknown
    // signature" 并可能阻塞。ApiCat 不是无障碍工具，直接关闭桥。
    if std::env::var_os("NO_AT_BRIDGE").is_none() {
        std::env::set_var("NO_AT_BRIDGE", "1");
    }
    // 外部显式设置优先（临时排查用），不被用户配置覆盖
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_some()
        || std::env::var_os("WEBKIT_DISABLE_COMPOSITING_MODE").is_some()
    {
        return;
    }
    match read_linux_webkit_renderer().as_str() {
        "auto" => {}
        "shm" => {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
        _ => {
            // software（默认）：光关 DMABUF/COMPOSITING 实测 WebKitWebProcess 仍会
            // 打开 /dev/dri/renderD128 向 i915 提交命令，整机 GPU HANG 照旧复现。
            // 再让 Mesa 走 llvmpipe 软件光栅化，才能真的不碰 GPU。
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
            if std::env::var_os("LIBGL_ALWAYS_SOFTWARE").is_none() {
                std::env::set_var("LIBGL_ALWAYS_SOFTWARE", "1");
            }
        }
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    setup_linux_font_rendering();
    #[cfg(target_os = "linux")]
    setup_linux_webview();

    apicat_lib::run()
}
