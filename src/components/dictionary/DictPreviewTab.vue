<template>
  <div class="dict-preview-tab">
    <!-- 未选中字典 -->
    <div v-if="!store.selectedDict" class="dict-preview-empty">
      <n-empty description="← 在左侧选择一个字典，预览其条目与被引用情况" size="small" />
    </div>

    <template v-else>
      <!-- 头部 -->
      <div class="preview-head">
        <div class="preview-title">
          <span class="preview-code">{{ store.selectedDict.code }}</span>
          <span class="preview-name">{{ store.selectedDict.name }}</span>
          <n-tag size="small" :bordered="false" type="info">{{ items.length }} 项</n-tag>
          <n-tag v-if="store.selectedDict.builtin === 1" size="small" :bordered="false">内置</n-tag>
        </div>
        <div class="preview-actions">
          <n-button size="small" secondary :disabled="!bindingCount" @click="gotoRules">
            查看绑定规则{{ bindingCount ? `（${bindingCount}）` : '' }}
          </n-button>
        </div>
      </div>

      <!-- 条目表格（结构化预览，替代裸 JSON）-->
      <div class="preview-section">
        <div class="section-title">字典条目</div>
        <div v-if="!items.length" class="section-empty">该字典暂无条目</div>
        <table v-else class="item-table">
          <thead>
            <tr>
              <th class="col-value">value</th>
              <th class="col-label">label</th>
              <th class="col-desc">description</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="item in items" :key="item.id">
              <td class="col-value">
                <code>{{ item.value }}</code>
              </td>
              <td class="col-label">{{ item.label }}</td>
              <td class="col-desc">{{ item.description || '—' }}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- 引用列表：谁在用这个字典（改条目前评估影响面）-->
      <div class="preview-section">
        <div class="section-title">被以下绑定引用</div>
        <div v-if="!bindingCount" class="section-empty">
          暂无字段绑定引用此字典；可在接口参数行点 📖 绑定
        </div>
        <template v-else>
          <div v-for="r in bindings.rules" :key="`r-${r.id}`" class="bind-row">
            <n-tag size="tiny" :bordered="false" type="success">项目级</n-tag>
            <code class="bind-field">{{ r.field_name }}</code>
            <span class="bind-hint">字段名（全项目）</span>
          </div>
          <div v-for="o in bindings.overrides" :key="`o-${o.id}`" class="bind-row">
            <n-tag size="tiny" :bordered="false" type="warning">接口级</n-tag>
            <code class="bind-field">{{ o.field_name }}</code>
            <span class="bind-hint">{{ requestLabel(o.request_id) }}</span>
          </div>
        </template>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NButton, NEmpty, NTag } from 'naive-ui'
import { useDictionaryStore } from '../../stores/dictionary'
import { useRequestStore } from '../../stores/request'

const emit = defineEmits<{
  'goto-rules': [dictId: number]
}>()

const store = useDictionaryStore()
const requestStore = useRequestStore()

const items = computed(() => store.itemsMap[store.selectedDict?.id ?? 0] ?? [])
const bindings = computed(() =>
  store.selectedDict ? store.bindingsForDict(store.selectedDict.id) : { rules: [], overrides: [] },
)
const bindingCount = computed(() => bindings.value.rules.length + bindings.value.overrides.length)

/** 接口级例外的宿主接口名：所在 collection 未加载时降级显示 id */
function requestLabel(requestId: number): string {
  const req = requestStore.findRequestById(requestId)
  return req ? `仅接口「${req.name}」` : `仅接口 #${requestId}`
}

function gotoRules() {
  if (store.selectedDict) emit('goto-rules', store.selectedDict.id)
}
</script>

<style scoped>
.dict-preview-tab {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
  overflow-y: auto;
  flex: 1;
  min-height: 0;
  height: 100%;
}

.dict-preview-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  flex: 1;
}

.preview-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-sm);
  flex-shrink: 0;
}
.preview-title {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  min-width: 0;
}
.preview-code {
  font-family: monospace;
  font-size: var(--font-size-lg);
  font-weight: 700;
  color: var(--color-primary);
}
.preview-name {
  font-size: var(--font-size-base);
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.preview-actions {
  display: flex;
  gap: var(--spacing-xs);
  flex-shrink: 0;
}

.preview-section {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}
.section-title {
  font-size: var(--font-size-sm);
  font-weight: 600;
  color: var(--text-tertiary);
}
.section-empty {
  font-size: var(--font-size-sm);
  color: var(--text-tertiary);
  padding: var(--spacing-xs) 0;
}

/* 条目表格 */
.item-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--font-size-sm);
}
.item-table th,
.item-table td {
  text-align: left;
  padding: var(--spacing-xs) var(--spacing-sm);
  border-bottom: 1px solid var(--border-base);
}
.item-table th {
  color: var(--text-tertiary);
  font-weight: 600;
  border-bottom-width: 2px;
}
.item-table .col-value {
  width: 120px;
  font-family: monospace;
}
.item-table .col-value code {
  color: var(--color-primary);
  background: var(--bg-hover);
  border-radius: 3px;
  padding: 0 4px;
}
.item-table .col-label {
  width: 30%;
  color: var(--text-primary);
}
.item-table .col-desc {
  color: var(--text-tertiary);
}

/* 引用列表 */
.bind-row {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  padding: var(--spacing-xs) var(--spacing-sm);
  border: 1px solid var(--border-base);
  border-radius: var(--radius-sm);
  font-size: var(--font-size-sm);
}
.bind-field {
  font-family: monospace;
  color: var(--color-primary);
  font-weight: 600;
  flex-shrink: 0;
}
.bind-hint {
  color: var(--text-tertiary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
