/**
 * 更新提示弹窗的全局状态（1.0.10+）。
 *
 * 两个触发点：启动期检查（startupChecks）与 ⚙️ 手动检查（TopBar）。
 * 原先各自调 dialog.info 传纯文本，更新日志里的 Markdown 不会渲染，
 * 而且长日志会把弹窗撑出屏幕。现在统一投递到这里，由 UpdateDialog 渲染。
 */
import { ref } from 'vue'
import { defineStore } from 'pinia'
import type { Update } from '@tauri-apps/plugin-updater'

export const useUpdateStore = defineStore('update', () => {
  const pending = ref<Update | null>(null)
  const visible = ref(false)

  function show(update: Update) {
    pending.value = update
    visible.value = true
  }

  function close() {
    visible.value = false
    pending.value = null
  }

  return { pending, visible, show, close }
})
