<template>
  <n-modal
    :show="store.visible"
    preset="card"
    :title="`发现新版本 ${store.pending?.version ?? ''}`"
    :style="{ width: '680px', maxWidth: '92vw' }"
    :mask-closable="false"
    @update:show="onShowChange"
  >
    <div class="update-body">
      <!-- 更新日志是 Markdown（release notes），走与响应预览一致的渲染器 -->
      <MarkdownRenderer v-if="body" :body="body" />
      <p v-else class="update-empty">有新版本可用，是否立即更新并重启？</p>
    </div>
    <template #footer>
      <n-space justify="end">
        <n-button quaternary @click="store.close()">稍后再说</n-button>
        <n-button type="primary" :loading="installing" @click="doInstall">立即更新</n-button>
      </n-space>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { NModal, NButton, NSpace, useMessage } from 'naive-ui'
import { relaunch } from '@tauri-apps/plugin-process'
import MarkdownRenderer from '../response/formatters/MarkdownRenderer.vue'
import { useUpdateStore } from '../../stores/update'

const store = useUpdateStore()
const message = useMessage()
const installing = ref(false)

const body = computed(() => store.pending?.body ?? '')

function onShowChange(v: boolean) {
  if (!v) store.close()
}

async function doInstall() {
  const update = store.pending
  if (!update) return
  installing.value = true
  const tip = message.loading('正在下载更新，请稍候...', { duration: 0 })
  try {
    await update.downloadAndInstall()
    tip.destroy()
    message.success('更新完成，正在重启...')
    setTimeout(() => relaunch(), 1500)
  } catch (e) {
    tip.destroy()
    message.error(`更新失败：${e}`)
  } finally {
    installing.value = false
  }
}
</script>

<style scoped>
/*
 * 更新日志限高滚动：release notes 动辄几百行，不限高会把弹窗撑出屏幕。
 * MarkdownRenderer 自带 flex:1 + overflow:auto，嵌在这里会和外层抢滚动条，
 * 所以覆盖成跟随外层容器。
 */
.update-body {
  max-height: 45vh;
  overflow-y: auto;
  padding-right: 4px;
}
.update-body :deep(.markdown-preview-wrapper) {
  flex: none;
  min-height: auto;
  overflow: visible;
  padding: 0;
  background: transparent;
  border-radius: 0;
}
.update-empty {
  margin: 0;
  color: var(--text-secondary);
}
</style>
