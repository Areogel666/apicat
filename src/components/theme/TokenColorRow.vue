<template>
  <div class="token-row">
    <div
      class="swatch"
      :style="{ background: currentValue }"
      @click="openColorPicker"
    />
    <div class="label">
      <span class="name">{{ tokenKey }}</span>
      <span class="desc">{{ description }}</span>
    </div>
    <span class="value">{{ currentValue }}</span>
    <button
      v-if="isModified"
      class="reset-btn"
      title="重置为默认值"
      @click="$emit('reset')"
    >↺</button>
    <!-- 隐藏的 color input -->
    <input
      ref="colorInputRef"
      type="color"
      :value="currentValue"
      class="hidden-input"
      @change="onColorChange"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'

const props = defineProps<{
  tokenKey: string
  currentValue: string
  defaultValue: string
  description: string
}>()

const emit = defineEmits<{
  'update:value': [val: string]
  'reset': []
}>()

const colorInputRef = ref<HTMLInputElement | null>(null)

const isModified = computed(() => props.currentValue !== props.defaultValue)

function openColorPicker() {
  colorInputRef.value?.click()
}

function onColorChange(e: Event) {
  const target = e.target as HTMLInputElement
  emit('update:value', target.value)
}
</script>

<style scoped>
.token-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 5px 12px;
  border-radius: var(--radius-sm);
  font-size: 12px;
}
.token-row:hover {
  background: var(--bg-hover);
}
.swatch {
  width: 22px;
  height: 22px;
  border-radius: 4px;
  border: 1.5px solid var(--border-base);
  flex-shrink: 0;
  cursor: pointer;
}
.label {
  flex: 1;
  min-width: 0;
}
.label .name {
  color: var(--text-primary);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  display: block;
}
.label .desc {
  color: var(--text-tertiary);
  font-size: 10px;
}
.value {
  color: var(--text-secondary);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  width: 68px;
  text-align: right;
  flex-shrink: 0;
}
.reset-btn {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: transparent;
  border: 1px solid var(--border-base);
  color: var(--text-tertiary);
  font-size: 10px;
  cursor: pointer;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
}
.reset-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
.hidden-input {
  position: absolute;
  opacity: 0;
  width: 0;
  height: 0;
  pointer-events: none;
}
</style>
