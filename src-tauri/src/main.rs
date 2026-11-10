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
#[cfg(target_os = "linux")]
fn read_linux_webkit_renderer() -> String {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|h| std::path::PathBuf::from(h).join(".local").join("share"))
        });
    let Some(base) = base else { return "shm".into() };
    let path = base.join("com.apicat.app").join("app-settings.json");
    let Ok(text) = std::fs::read_to_string(path) else { return "shm".into() };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else { return "shm".into() };
    json.get("linux.webkitRenderer")
        .and_then(|v| v.as_str())
        .unwrap_or("shm")
        .to_string()
}

#[cfg(target_os = "linux")]
fn setup_linux_webview() {
    // 外部显式设置优先（临时排查用），不被用户配置覆盖
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_some()
        || std::env::var_os("WEBKIT_DISABLE_COMPOSITING_MODE").is_some()
    {
        return;
    }
    match read_linux_webkit_renderer().as_str() {
        "auto" => {}
        "software" => {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        }
        _ => {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
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
