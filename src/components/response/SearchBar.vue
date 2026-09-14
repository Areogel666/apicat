<template>
  <div class="text-search-bar" :class="{ 'has-open': open }">
    <n-input
      v-if="open"
      ref="inputRef"
      v-model:value="keyword"
      size="tiny"
      placeholder="在响应中搜索…"
      clearable
      style="width: 180px"
      @update:value="onInput"
      @keydown.enter.prevent="goNext"
      @keydown.esc.prevent="close"
    />
    <template v-else>
      <button class="text-search-btn" title="搜索" @click="open = true; focusInput()">🔍</button>
    </template>
    <template v-if="open">
      <span class="text-search-count">{{ countText || '0' }}</span>
      <button class="text-search-btn" title="上一个 (Shift+Enter)" @click="goPrev">▲</button>
      <button class="text-search-btn" title="下一个 (Enter)" @click="goNext">▼</button>
      <button class="text-search-btn" title="关闭 (Esc)" @click="close">✕</button>
    </template>
  </div>
</template>

<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'
import { NInput } from 'naive-ui'

/**
 * 自实现文本搜索条（Linux webkit 无原生 find 时的兜底）。
 * 通过 v-model:keyword 双向同步搜索词；matches 触发父组件执行高亮。
 */
const props = defineProps<{
  open: boolean
  keyword: string
  countText: string
}>()

const emit = defineEmits<{
  'update:keyword': [v: string]
  'update:open': [v: boolean]
  'go-prev': []
  'go-next': []
  'close': []
}>()

const inputRef = ref<InstanceType<typeof NInput> | null>(null)

const open = ref(props.open)
const keyword = ref(props.keyword)

// 双向同步：开/关、关键词
watch(open, (v) => emit('update:open', v))
watch(keyword, (v) => emit('update:keyword', v))
watch(() => props.open, (v) => { open.value = v })
watch(() => props.keyword, (v) => { keyword.value = v })

function focusInput() {
  nextTick(() => inputRef.value?.focus())
}
function onInput(_v: string) {
  // 词变化交由父组件 search
}
function goNext() {
  if (!keyword.value.trim()) return
  emit('go-next')
}
function goPrev() {
  if (!keyword.value.trim()) return
  emit('go-prev')
}
function close() {
  open.value = false
  emit('close')
}
</script>

<style scoped>
.text-search-bar {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 2px 4px;
}
.text-search-btn {
  border: none;
  background: transparent;
  cursor: pointer;
  font-size: 12px;
  padding: 1px 4px;
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
}
.text-search-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
.text-search-count {
  font-size: 12px;
  color: var(--text-tertiary);
  min-width: 34px;
  text-align: center;
}
</style>