<template>
  <div class="section">
    <div class="section-title">预设主题</div>
    <div class="preset-cards">
      <div
        v-for="preset in PRESET_THEMES"
        :key="preset.id"
        class="preset-card"
        :class="{ active: activePresetId === preset.id }"
        @click="selectPreset(preset)"
      >
        <div class="card-swatches">
          <span
            v-for="(sw, i) in preset.swatches"
            :key="i"
            class="swatch-dot"
            :style="{ background: sw }"
          />
        </div>
        <div class="card-name">{{ preset.name }}</div>
        <div class="card-desc">{{ preset.description }}</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { PRESET_THEMES, type ThemePreset } from './presets'
import { useThemeStore } from '../../stores/theme'

const themeStore = useThemeStore()
const activePresetId = ref<string | null>(null)

function selectPreset(preset: ThemePreset) {
  activePresetId.value = preset.id
  // 预设只定主色/语义色，不锁定深浅模式（背景色跟随当前模式）
  themeStore.customTokens = { ...preset.tokens }
  themeStore.density = preset.density
  themeStore.radiusScale = preset.radiusScale
  themeStore.fontSize = preset.fontSize
  themeStore.applyTheme()
}
</script>

<style scoped>
.preset-cards {
  display: flex;
  gap: 8px;
}
.preset-card {
  flex: 1;
  padding: 10px;
  border-radius: var(--radius-md);
  border: 2px solid var(--border-base);
  cursor: pointer;
  text-align: center;
  font-size: 12px;
  background: var(--bg-elevated);
  transition: border-color 0.15s;
}
.preset-card:hover {
  border-color: var(--text-tertiary);
}
.preset-card.active {
  border-color: var(--color-primary);
}
.card-swatches {
  display: flex;
  gap: 3px;
  justify-content: center;
  margin-bottom: 6px;
}
.swatch-dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  border: 1px solid rgba(128,128,128,0.2);
}
.card-name {
  font-weight: 500;
  color: var(--text-primary);
}
.card-desc {
  color: var(--text-tertiary);
  font-size: 10px;
  margin-top: 2px;
}
</style>
