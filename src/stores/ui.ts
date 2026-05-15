import { defineStore } from 'pinia'
import { ref } from 'vue'

/**
 * 通用 UI 状态。
 *
 * 历史：1.0.x 早期此 store 含 darkMode 字段，M3-B 主题底座引入 themeStore 后移除。
 * 当前仅保留 sidebarWidth（Sidebar.vue 引用）。
 */
export const useUiStore = defineStore('ui', () => {
  // 侧边栏宽度（可拖拽调整，M2 实现拖拽，M1 先固定）
  const sidebarWidth = ref(240)

  return { sidebarWidth }
})
