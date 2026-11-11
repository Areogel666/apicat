import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { readSetting, writeSetting, flushSettings } from './_persistedSettings'

/** 默认勾选三种主流算法；zstd 默认不勾（服务端普及度低） */
export const DEFAULT_DECOMPRESS_ALGORITHMS = ['gzip', 'deflate', 'br']

/**
 * 网络相关设置（当前只有「响应解压」）。
 * 持久化到 app-settings.json，Rust 侧启动时读取；保存后调用
 * `apply_network_settings` 让主 HTTP client 即时重建。
 */
export const useNetworkStore = defineStore('network', () => {
  const decompressEnabled = ref(false)
  const decompressAlgorithms = ref<string[]>([...DEFAULT_DECOMPRESS_ALGORITHMS])

  let loaded = false

  async function load() {
    if (loaded) return
    const enabled = await readSetting<boolean>('network.decompressEnabled')
    const algorithms = await readSetting<string[]>('network.decompressAlgorithms')
    decompressEnabled.value = enabled ?? false
    decompressAlgorithms.value = Array.isArray(algorithms)
      ? algorithms
      : [...DEFAULT_DECOMPRESS_ALGORITHMS]
    loaded = true
  }

  async function save() {
    await writeSetting('network.decompressEnabled', decompressEnabled.value)
    await writeSetting('network.decompressAlgorithms', decompressAlgorithms.value)
    // 压测启动时 Rust 直接读 app-settings.json，先强制落盘避免读到旧值
    await flushSettings()
    // 直接传值给 Rust，避免 plugin-store 自动落盘防抖造成读旧文件
    await invoke('apply_network_settings', {
      enabled: decompressEnabled.value,
      algorithms: decompressAlgorithms.value,
    })
  }

  return { decompressEnabled, decompressAlgorithms, load, save }
})
