/**
 * 复制文本到剪贴板。
 *
 * Tauri 环境下 navigator.clipboard 可能因权限策略抛错，
 * 统一在此降级为 execCommand，调用方无需各写一份 try/catch。
 */
export async function copyText(text: string): Promise<void> {
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
