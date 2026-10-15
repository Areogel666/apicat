import { writeText } from '@tauri-apps/plugin-clipboard-manager'

/**
 * 复制文本到剪贴板。
 *
 * 优先走 Tauri 原生剪贴板插件：Linux WebKitGTK 下 navigator.clipboard 会
 * 静默不生效（promise resolve 但内容未写入），execCommand 依赖同步用户手势，
 * 均不可靠；插件走各平台原生剪贴板（GTK / Win32 / macOS）。
 * 插件不可用（如纯浏览器 dev）时降级 Web API，调用方无需各写一份 try/catch。
 */
export async function copyText(text: string): Promise<void> {
  try {
    await writeText(text)
    return
  } catch {
    // 非 Tauri 环境，降级 Web API
  }
  try {
    await navigator.clipboard.writeText(text)
  } catch {
    const ta = document.createElement('textarea')
    ta.value = text
    document.body.appendChild(ta)
    ta.select()
    document.execCommand('copy')
    document.body.removeChild(ta)
  }
}
