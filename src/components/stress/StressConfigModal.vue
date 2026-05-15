<template>
  <n-modal
    v-model:show="show"
    preset="dialog"
    title="压测配置"
    :show-icon="false"
    style="width: 460px"
    positive-text="开始压测"
    negative-text="取消"
    @positive-click="handleStart"
    @negative-click="show = false"
  >
    <div class="stress-config">
      <!-- 用例选择（有用例时展示）-->
      <div v-if="(testCases ?? []).length > 0" class="config-row config-row--col">
        <span class="config-label">请求参数</span>
        <n-radio-group v-model:value="paramSource" size="small" style="flex-direction:column; gap:6px">
          <n-radio value="current">使用当前编辑区参数</n-radio>
          <n-radio value="testcase">使用测试用例参数</n-radio>
        </n-radio-group>
        <n-select
          v-if="paramSource === 'testcase'"
          v-model:value="selectedTestCaseId"
          :options="testCaseOptions"
          placeholder="选择用例"
          size="small"
          style="margin-top:4px; width:100%"
        />
      </div>

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
          :max="config.mode === 'count' ? 10000 : 3600"
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
import { ref, reactive, computed } from 'vue'
import { NModal, NInputNumber, NRadioGroup, NRadio, NSelect } from 'naive-ui'
import type { StressConfig, TestCase } from '../../types'

const show = defineModel<boolean>('show', { required: true })

const props = defineProps<{
  testCases?: TestCase[]
}>()

const emit = defineEmits<{
  start: [config: StressConfig, testCaseId: number | null]
}>()

const config = reactive<StressConfig>({
  concurrent: 10,
  mode: 'count',
  value: 100,
})

const paramSource = ref<'current' | 'testcase'>('current')
const selectedTestCaseId = ref<number | null>(null)

const testCaseOptions = computed(() =>
  (props.testCases ?? []).map(tc => ({
    label: tc.name || `用例 #${tc.id}`,
    value: tc.id,
  }))
)

function handleStart() {
  const tcId = paramSource.value === 'testcase' ? selectedTestCaseId.value : null
  emit('start', { ...config }, tcId)
  show.value = false
}
</script>

<style scoped>
.stress-config {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 8px 0;
}

.config-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.config-row--col {
  flex-direction: column;
  align-items: flex-start;
  gap: 6px;
}

.config-label {
  width: 110px;
  font-size: 13px;
  color: var(--text-primary);
  flex-shrink: 0;
  font-weight: 500;
}

.config-hint {
  font-size: 12px;
  color: var(--text-tertiary);
}

.config-tip {
  font-size: 12px;
  color: var(--text-tertiary);
  padding: 4px 8px;
  background: var(--bg-elevated);
  border-radius: 4px;
}
</style>
