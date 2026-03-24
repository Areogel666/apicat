<template>
  <n-modal
    v-model:show="show"
    preset="dialog"
    title="压测配置"
    :show-icon="false"
    style="width: 420px"
    positive-text="开始压测"
    negative-text="取消"
    @positive-click="handleStart"
    @negative-click="show = false"
  >
    <div class="stress-config">
      <!-- 并发数 -->
      <div class="config-row">
        <span class="config-label">并发数</span>
        <n-input-number
          v-model:value="config.concurrent"
          :min="1"
          :max="500"
          size="small"
          style="width: 120px"
        />
        <span class="config-hint">1 ~ 500</span>
      </div>

      <!-- 模式选择 -->
      <div class="config-row">
        <span class="config-label">模式</span>
        <n-radio-group v-model:value="config.mode" size="small">
          <n-radio value="count">总请求数</n-radio>
          <n-radio value="duration">持续时间</n-radio>
        </n-radio-group>
      </div>

      <!-- 数值 -->
      <div class="config-row">
        <span class="config-label">
          {{ config.mode === 'count' ? '总请求数' : '持续时间（秒）' }}
        </span>
        <n-input-number
          v-model:value="config.value"
          :min="1"
          :max="config.mode === 'count' ? 100000 : 3600"
          size="small"
          style="width: 120px"
        />
        <span class="config-hint">
          {{ config.mode === 'count' ? '条' : '秒' }}
        </span>
      </div>

      <!-- 预估提示 -->
      <div class="config-tip" v-if="config.mode === 'count'">
        💡 {{ config.concurrent }} 并发 × {{ config.value }} 请求
      </div>
      <div class="config-tip" v-else>
        💡 {{ config.concurrent }} 并发持续 {{ config.value }} 秒
      </div>
    </div>
  </n-modal>
</template>

<script setup lang="ts">
import { reactive } from 'vue'
import { NModal, NInputNumber, NRadioGroup, NRadio } from 'naive-ui'
import type { StressConfig } from '../../types'

const show = defineModel<boolean>('show', { required: true })

const emit = defineEmits<{
  start: [config: StressConfig]
}>()

const config = reactive<StressConfig>({
  concurrent: 10,
  mode: 'count',
  value: 100,
})

function handleStart() {
  emit('start', { ...config })
  show.value = false
}
</script>

<style scoped>
.stress-config {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 8px 0;
}

.config-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.config-label {
  width: 110px;
  font-size: 13px;
  color: var(--n-text-color, #333);
  flex-shrink: 0;
}

.config-hint {
  font-size: 12px;
  color: var(--n-text-color-3, #999);
}

.config-tip {
  font-size: 12px;
  color: var(--n-text-color-3, #999);
  padding: 4px 8px;
  background: var(--n-color-modal, #f9f9f9);
  border-radius: 4px;
}
</style>
