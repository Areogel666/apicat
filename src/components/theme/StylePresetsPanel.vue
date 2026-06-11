<template>
  <div class="style-presets">
    <!-- 布局密度 -->
    <div class="preset-row">
      <div class="preset-label">布局密度</div>
      <div class="density-options">
        <button
          v-for="opt in densityOptions"
          :key="opt.value"
          class="density-btn"
          :class="{ active: themeStore.density === opt.value }"
          @click="themeStore.density = opt.value; themeStore.applyTheme()"
        >{{ opt.label }}</button>
      </div>
    </div>

    <!-- 圆角 -->
    <div class="preset-row">
      <div class="preset-label">圆角</div>
      <div class="tab-group">
        <button
          v-for="opt in radiusOptions"
          :key="opt.value"
          class="tab-btn"
          :class="{ active: themeStore.radiusScale === opt.value }"
          @click="themeStore.radiusScale = opt.value; themeStore.applyTheme()"
        >{{ opt.label }}</button>
      </div>
    </div>

    <!-- 字体大小 -->
    <div class="preset-row">
      <div class="preset-label">字体大小</div>
      <div class="font-options">
        <button
          v-for="opt in fontSizeOptions"
          :key="opt.value"
          class="font-btn"
          :class="{ active: themeStore.fontSize === opt.value }"
          @click="themeStore.fontSize = opt.value; themeStore.applyTheme()"
        >{{ opt.label }}</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useThemeStore } from '../../stores/theme'

const themeStore = useThemeStore()

const densityOptions = [
  { label: '紧凑', value: 'compact' as const },
  { label: '默认', value: 'default' as const },
  { label: '宽松', value: 'spacious' as const },
]

const radiusOptions = [
  { label: '小', value: 0.5 as const },
  { label: '中', value: 1.0 as const },
  { label: '大', value: 1.5 as const },
]

const fontSizeOptions = [
  { label: 'S', value: 's' as const },
  { label: 'M', value: 'm' as const },
  { label: 'L', value: 'l' as const },
]
</script>

<style scoped>
.style-presets {
  display: flex;
  gap: 20px;
  align-items: flex-start;
}
.preset-row {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.preset-label {
  font-size: 11px;
  color: var(--text-secondary);
  margin-bottom: 2px;
}
.density-options {
  display: flex;
  gap: 2px;
}
.density-btn {
  padding: 3px 12px;
  border-radius: 99px;
  border: 1px solid var(--border-base);
  background: transparent;
  color: var(--text-secondary);
  font-size: 11px;
  cursor: pointer;
  white-space: nowrap;
}
.density-btn.active {
  background: var(--color-primary);
  color: #000;
  border-color: var(--color-primary);
  font-weight: 500;
}
.tab-group {
  display: flex;
  gap: 0;
}
.tab-btn {
  padding: 3px 12px;
  border: 1px solid var(--border-base);
  background: transparent;
  color: var(--text-secondary);
  font-size: 11px;
  cursor: pointer;
  margin-left: -1px;
}
.tab-btn:first-child {
  border-radius: var(--radius-sm) 0 0 var(--radius-sm);
  margin-left: 0;
}
.tab-btn:last-child {
  border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
}
.tab-btn.active {
  background: var(--color-primary);
  color: #000;
  border-color: var(--color-primary);
}
.font-options {
  display: flex;
  gap: 4px;
}
.font-btn {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  border: 2px solid var(--border-base);
  background: var(--bg-elevated);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
  font-size: 11px;
  color: var(--text-secondary);
}
.font-btn.active {
  border-color: var(--color-primary);
  color: var(--color-primary);
}
</style>
