<template>
  <div
    class="resizable-splitter"
    :class="[direction]"
    @mousedown="onMouseDown"
  >
    <div class="splitter-line" />
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

/**
 * 通用分栏拖拽分隔条
 *
 * props:
 *   direction: 'horizontal'（左右拖，调宽度）| 'vertical'（上下拖，调高度）
 *   minSize:  受压栏的最小尺寸（px），默认 120
 *   maxSize:  受压栏的最大尺寸（px），默认 Infinity
 *   defaultSize: 默认受压栏尺寸（px），默认 280
 *   storageKey?: 可选持久化 key（存到 localStorage，重启保留）
 *
 * emits:
 *   resize: (newSize: number) — 拖拽结束时发射最终尺寸
 *   resizing: (newSize: number) — 拖拽过程中实时发射
 */

const props = withDefaults(defineProps<{
  direction: 'horizontal' | 'vertical'
  minSize?: number
  maxSize?: number
  defaultSize?: number
  storageKey?: string
}>(), {
  minSize: 120,
  maxSize: Infinity,
  defaultSize: 280,
})

const emit = defineEmits<{
  resize: [size: number]
  resizing: [size: number]
}>()

const isDragging = ref(false)

function onMouseDown(e: MouseEvent) {
  e.preventDefault()
  isDragging.value = true

  const startPos = props.direction === 'horizontal' ? e.clientX : e.clientY
  const startSize = props.defaultSize

  const bodyStyle = document.body.style
  bodyStyle.cursor = props.direction === 'horizontal' ? 'col-resize' : 'row-resize'
  bodyStyle.userSelect = 'none'

  function onMove(ev: MouseEvent) {
    if (!isDragging.value) return
    const currentPos = props.direction === 'horizontal' ? ev.clientX : ev.clientY
    const delta = currentPos - startPos
    let newSize: number
    if (props.direction === 'horizontal') {
      newSize = startSize + delta
    } else {
      newSize = startSize - delta
    }
    newSize = Math.max(props.minSize, Math.min(props.maxSize, newSize))
    emit('resizing', newSize)
  }

  function onUp(ev: MouseEvent) {
    isDragging.value = false
    document.removeEventListener('mousemove', onMove)
    document.removeEventListener('mouseup', onUp)
    bodyStyle.cursor = ''
    bodyStyle.userSelect = ''

    const currentPos = props.direction === 'horizontal' ? ev.clientX : ev.clientY
    const delta = currentPos - startPos
    let finalSize: number
    if (props.direction === 'horizontal') {
      finalSize = startSize + delta
    } else {
      finalSize = startSize - delta
    }
    finalSize = Math.max(props.minSize, Math.min(props.maxSize, finalSize))

    emit('resize', finalSize)

    if (props.storageKey) {
      localStorage.setItem(props.storageKey, String(finalSize))
    }
  }

  document.addEventListener('mousemove', onMove)
  document.addEventListener('mouseup', onUp)
}
</script>

<style scoped>
.resizable-splitter {
  position: relative;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  transition: background 0.15s;
  z-index: 10;
}
.resizable-splitter:hover {
  background: rgba(128, 128, 128, 0.08);
}
.resizable-splitter.horizontal {
  width: 4px;
  cursor: col-resize;
  height: 100%;
}
.resizable-splitter.vertical {
  height: 4px;
  cursor: row-resize;
  width: 100%;
}
.splitter-line {
  width: 100%;
  height: 100%;
  opacity: 0;
}
.resizable-splitter:hover .splitter-line {
  opacity: 1;
  background: var(--border-strong);
  border-radius: 1px;
}
.resizable-splitter.horizontal .splitter-line {
  width: 1.5px;
  height: 40px;
}
.resizable-splitter.vertical .splitter-line {
  width: 40px;
  height: 1.5px;
}
</style>
