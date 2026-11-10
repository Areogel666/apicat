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

    <!-- 玻璃质感（1.0.9）：独立于预设的手动开关 + 模糊/透明度档位 -->
    <div class="preset-row">
      <div class="preset-label">玻璃质感</div>
      <div class="glass-row">
        <button
          class="density-btn"
          :class="{ active: themeStore.glass }"
          :title="themeStore.glass ? '点击关闭玻璃质感' : '卡片与弹窗变为半透明毛玻璃'"
          @click="themeStore.glass = !themeStore.glass; themeStore.applyTheme()"
        >{{ themeStore.glass ? '开启中' : '关闭' }}</button>
        <div class="tab-group glass-group" :class="{ disabled: !themeStore.glass }">
          <button
            class="tab-btn"
            :class="{ active: !themeStore.glassBlurEnabled }"
            :disabled="!themeStore.glass"
            title="关闭背景模糊（保留半透明玻璃质感）"
            @click="themeStore.glassBlurEnabled = false; themeStore.applyTheme()"
          >关</button>
          <button
            v-for="opt in blurOptions"
            :key="opt.value"
            class="tab-btn"
            :class="{ active: themeStore.glassBlurEnabled && themeStore.glassBlur === opt.value }"
            :disabled="!themeStore.glass"
            :title="`背景模糊 ${opt.px}px`"
            @click="themeStore.glassBlurEnabled = true; themeStore.glassBlur = opt.value; themeStore.applyTheme()"
          >{{ opt.label }}</button>
        </div>
        <div class="tab-group glass-group" :class="{ disabled: !themeStore.glass }">
          <button
            v-for="opt in alphaOptions"
            :key="opt.value"
            class="tab-btn"
            :class="{ active: themeStore.glassAlpha === opt.value }"
            :disabled="!themeStore.glass"
            :title="`表面不透明度 ${opt.alpha}`"
            @click="themeStore.glassAlpha = opt.value; themeStore.applyTheme()"
          >{{ opt.label }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useThemeStore, type GlassLevel } from '../../stores/theme'

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

const blurOptions: { label: string; value: GlassLevel; px: number }[] = [
  { label: '轻', value: 'light', px: 8 },
  { label: '中', value: 'medium', px: 14 },
  { label: '重', value: 'heavy', px: 22 },
]

const alphaOptions: { label: string; value: GlassLevel; alpha: string }[] = [
  { label: '更透', value: 'light', alpha: '62%' },
  { label: '标准', value: 'medium', alpha: '72%' },
  { label: '更实', value: 'heavy', alpha: '85%' },
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
  color: #fff;
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
  color: #fff;
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

/* 玻璃质感行：开关 + 两组档位并排；关闭态档位整体淡出且不可点 */
.glass-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.glass-group.disabled {
  opacity: 0.45;
  pointer-events: none;
}
.tab-btn:disabled {
  cursor: default;
}
</style>
