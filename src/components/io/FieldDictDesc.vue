<template>
  <div class="fdd">
    <!-- 有绑定字典 → 命中/未匹配 + Tooltip 全枚举 + 例外角标 -->
    <template v-if="dictionary">
      <span :class="['fdd-tag', hit ? 'fdd-hit' : 'fdd-miss']" :title="dictionary.code">
        {{ hit ? `${hit.value} = ${hit.label}` : '未匹配' }}
      </span>
      <n-tooltip trigger="hover" placement="bottom-end">
        <template #trigger>
          <span class="fdd-more" title="查看字典全部枚举">{{ dictionary.code }}</span>
        </template>
        <div class="fdd-enums">
          <div
            v-for="it in items"
            :key="it.id"
            :class="['fdd-enum-row', isHit(it) && 'fdd-enum-hit']"
          >
            {{ it.value }} = {{ it.label }}{{ it.description ? ' — ' + it.description : '' }}
            <span v-if="isHit(it)" class="fdd-enum-mark">◂ 命中</span>
          </div>
        </div>
      </n-tooltip>
      <span v-if="overridden" class="fdd-ovr" title="此接口已对该字段换绑/解绑，例外优先于项目规则">ⓘ</span>
    </template>
    <!-- 手写描述：始终标记追加在尾部（有字典时跟在枚举信息后）；超长省略 + 悬停看全文 -->
    <n-tooltip v-if="manual && manualTruncated" trigger="hover" placement="top-end">
      <template #trigger>
        <span ref="manualRef" class="fdd-manual">◇ 手写：{{ manual }}</span>
      </template>
      <div class="fdd-manual-full">{{ manual }}</div>
    </n-tooltip>
    <span v-else-if="manual" ref="manualRef" class="fdd-manual">◇ 手写：{{ manual }}</span>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { NTooltip } from 'naive-ui'
import { useDictionaryStore } from '../../stores/dictionary'
import { useRequestStore } from '../../stores/request'
import { looseEq } from '../response/formatters/jsonTreeUtils'
import type { DictionaryItem } from '../../types'

const props = withDefaults(defineProps<{
  /** 字段名（key）—— 决定绑定哪个字典 */
  field: string
  /** 字段当前值 —— 决定命中哪个字典项 */
  value: string
  /** 手写描述（如存在，标记追加在尾部） */
  manual?: string
}>(), { manual: '' })

const dictStore = useDictionaryStore()
const requestStore = useRequestStore()

const dictId = computed(() => dictStore.dictIdForField(props.field, requestStore.activeRequestId))
const dictionary = computed(() => dictStore.dictById(dictId.value))
const items = computed<DictionaryItem[]>(() => (dictId.value != null ? (dictStore.itemsMap[dictId.value] ?? []) : []))

function isHit(it: DictionaryItem): boolean {
  return looseEq(props.value, it.value)
}

const hit = computed(() => items.value.find(isHit) ?? null)

/** 该字段是否存在接口级例外（换绑/解绑） */
const overridden = computed(() => {
  if (requestStore.activeRequestId == null) return false
  return dictStore.fieldOverrides.some(o =>
    o.request_id === requestStore.activeRequestId && o.field_name === props.field)
})

// ── 手写描述截断检测：省略生效时才弹全文 Tooltip（不截断则普通展示，悬停不干扰） ──
const manualRef = ref<HTMLElement | null>(null)
const manualTruncated = ref(false)
watch(
  () => props.manual,
  async () => {
    await nextTick()
    const el = manualRef.value
    manualTruncated.value = !!el && el.scrollWidth > el.clientWidth + 1
  },
  { immediate: true },
)
</script>

<style scoped>
.fdd {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  min-width: 0;
  flex: 1;
  overflow: hidden;
}
.fdd-tag {
  font-size: var(--font-size-sm);
  padding: 0 var(--spacing-xs);
  border-radius: var(--radius-sm);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: min(160px, 100%);
  flex-shrink: 0;
}
.fdd-hit {
  background: var(--color-success-soft, rgba(24, 160, 88, 0.15));
  color: var(--color-success);
}
.fdd-miss {
  background: var(--bg-hover);
  color: var(--text-tertiary);
}
.fdd-more {
  font-size: var(--font-size-sm);
  color: var(--text-tertiary);
  cursor: help;
  flex-shrink: 0;
  text-decoration: underline dotted;
}
.fdd-manual {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  /* 允许收缩并参与省略号截断（flex-shrink:0 会保持内容宽、溢出被父级剪掉不出省略号） */
  flex-shrink: 1;
  min-width: 0;
}
.fdd-manual-full {
  max-width: 320px;
  font-size: var(--font-size-sm);
  color: var(--text-primary);
  word-break: break-all;
  white-space: normal;
}
.fdd-ovr {
  color: var(--color-warning);
  font-size: var(--font-size-sm);
  flex-shrink: 0;
}
.fdd-enums {
  /* 内容块自带主题背景与边框，避免依赖 Tooltip 气泡默认底色（浅/深都清晰） */
  background: var(--bg-elevated);
  border: 1px solid var(--border-base);
  border-radius: var(--radius-sm);
  padding: var(--spacing-xs) 0;
  /* 布局阈值：枚举较多时可滚动 */
  max-height: 280px;
  overflow-y: auto;
  font-size: var(--font-size-sm);
}
.fdd-enum-row {
  padding: var(--spacing-xs) var(--spacing-sm);
  /* 主文字色保证对比度（secondary 在浅色背景上过灰看不清） */
  color: var(--text-primary);
  white-space: nowrap;
}
.fdd-enum-mark {
  color: var(--color-success);
  margin-left: var(--spacing-sm);
}
</style>