import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useUiStore = defineStore('ui', () => {
  // 跟随系统暗色模式
  const darkMode = ref(window.matchMedia('(prefers-color-scheme: dark)').matches)

  // 侧边栏宽度（可拖拽调整，M2 实现拖拽，M1 先固定）
  const sidebarWidth = ref(240)

  function toggleDarkMode() {
    darkMode.value = !darkMode.value
  }

  return { darkMode, sidebarWidth, toggleDarkMode }
})
