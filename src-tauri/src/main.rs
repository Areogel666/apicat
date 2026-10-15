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

fn main() {
    #[cfg(target_os = "linux")]
    setup_linux_font_rendering();

    apicat_lib::run()
}
