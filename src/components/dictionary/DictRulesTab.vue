<template>
  <div class="dict-rules-tab">
    <!-- 工具栏：字典筛选 + 新建规则 -->
    <div class="rules-toolbar">
      <n-input
        v-model:value="search"
        size="small"
        clearable
        placeholder="搜索字段名 / 字典"
        class="rules-search"
      />
      <n-select
        v-model:value="dictFilter"
        :options="dictFilterOptions"
        size="small"
        clearable
        placeholder="按字典筛选"
        class="rules-dictfilter"
      />
      <n-button v-if="filterDictId != null" size="small" quaternary @click="clearDictFilter">
        清除字典筛选 ✕
      </n-button>
    </div>

    <!-- 筛选来源提示（从预览页跳转过来时）-->
    <div v-if="filterDictId != null" class="rules-filter-hint">
      正在筛选字典 <code>{{ dictNameOf(filterDictId) }}</code> 的绑定规则
    </div>

    <!-- 项目级规则 -->
    <div class="rules-section">
      <div class="section-title">
        项目级规则
        <n-tag size="tiny" :bordered="false" type="success">{{ filteredRules.length }}</n-tag>
        <span class="section-hint">字段名 → 字典，全项目生效</span>
      </div>
      <div v-if="!filteredRules.length" class="section-empty">
        {{ search || dictFilter != null ? '无匹配的项目级规则' : '暂无项目级规则' }}
      </div>
      <div v-for="r in filteredRules" :key="`r-${r.id}`" class="rule-row">
        <code class="rule-field">{{ r.field_name }}</code>
        <span class="rule-arrow">→</span>
        <span class="rule-dict">{{ dictNameOf(r.dictionary_id) }}</span>
        <n-button size="tiny" quaternary title="删除此项目级规则" @click="removeRule(r.field_name)">
          删除
        </n-button>
      </div>
    </div>

    <!-- 接口级例外 -->
    <div class="rules-section">
      <div class="section-title">
        接口级例外
        <n-tag size="tiny" :bordered="false" type="warning">{{ filteredOverrides.length }}</n-tag>
        <span class="section-hint">仅对指定接口生效，优先级高于项目级</span>
      </div>
      <div v-if="!filteredOverrides.length" class="section-empty">
        {{ search || dictFilter != null ? '无匹配的接口级例外' : '暂无接口级例外' }}
      </div>
      <div v-for="o in filteredOverrides" :key="`o-${o.id}`" class="rule-row">
        <code class="rule-field">{{ o.field_name }}</code>
        <span class="rule-arrow">→</span>
        <span class="rule-dict" :class="{ 'is-unbound': o.dictionary_id == null }">
          {{ o.dictionary_id == null ? '已解除绑定（本接口不命中字典）' : dictNameOf(o.dictionary_id) }}
        </span>
        <span class="rule-host">{{ requestLabel(o.request_id) }}</span>
        <n-button size="tiny" quaternary title="删除此接口级例外" @click="removeOverride(o)">
          删除
        </n-button>
      </div>
    </div>

    <!-- 新建项目级规则（页内直接建；接口级例外仍从接口编辑器 📖 绑定）-->
    <div class="rules-add">
      <div class="section-title">新建项目级规则</div>
      <div class="rules-add-row">
        <n-input
          v-model:value="newField"
          size="small"
          placeholder="字段名，如 status"
          class="rules-add-field"
        />
        <n-select
          v-model:value="newDict"
          :options="dictOptions"
          size="small"
          filterable
          placeholder="选择字典"
          class="rules-add-dict"
        />
        <n-button size="small" type="primary" ghost :disabled="!canAdd" @click="addRule">
          添加
        </n-button>
      </div>
      <div class="rules-add-hint">
        接口级例外请到接口参数行点 📖 绑定（可选「仅当前接口」作用域）
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { NButton, NInput, NSelect, NTag, useMessage } from 'naive-ui'
import { useDictionaryStore } from '../../stores/dictionary'
import { useProjectStore } from '../../stores/project'
import { useRequestStore } from '../../stores/request'
import type { FieldDictionaryOverride } from '../../types'

const props = defineProps<{
  /** 从字典预览页跳转过来时的字典筛选；null = 显示全部规则 */
  filterDictId: number | null
}>()

const emit = defineEmits<{
  'update:filterDictId': [id: number | null]
}>()

const store = useDictionaryStore()
const projectStore = useProjectStore()
const requestStore = useRequestStore()
const message = useMessage()

const currentProjectId = computed(() => projectStore.currentProjectId)

// ── 筛选状态 ───────────────────────────────────────────────
const search = ref('')
// 内部筛选（下拉选择）与外部传入的 filterDictId（跳转联动）取并集：
// 任一非空即生效，下拉优先展示外部传入值。
const internalDictFilter = ref<number | null>(null)
const dictFilter = computed<number | null>({
  get: () => props.filterDictId ?? internalDictFilter.value,
  set: (v) => {
    internalDictFilter.value = v
    // 用户主动改动下拉时，同步清掉外部传入的筛选，避免两处状态打架
    if (props.filterDictId !== null && v !== props.filterDictId) {
      emit('update:filterDictId', null)
    }
  },
})

const dictFilterId = computed(() => dictFilter.value)

function clearDictFilter() {
  internalDictFilter.value = null
  emit('update:filterDictId', null)
}

// ── 规则列表（两级）────────────────────────────────────────
const all = computed(() => store.allFieldBindings())

function matchSearch(fieldName: string, dictId: number | null): boolean {
  const kw = search.value.trim().toLowerCase()
  if (!kw) return true
  if (fieldName.toLowerCase().includes(kw)) return true
  if (dictId != null && dictNameOf(dictId).toLowerCase().includes(kw)) return true
  return false
}

const filteredRules = computed(() =>
  all.value.rules.filter(r => {
    if (dictFilterId.value != null && r.dictionary_id !== dictFilterId.value) return false
    return matchSearch(r.field_name, r.dictionary_id)
  }),
)

const filteredOverrides = computed(() =>
  all.value.overrides.filter(o => {
    if (dictFilterId.value != null && o.dictionary_id !== dictFilterId.value) return false
    return matchSearch(o.field_name, o.dictionary_id)
  }),
)

// ── 字典选项 ───────────────────────────────────────────────
const dictOptions = computed(() =>
  store.dictionaries.map(d => ({ label: `${d.code}（${d.name}）`, value: d.id })))
const dictFilterOptions = dictOptions

function dictNameOf(id: number): string {
  const d = store.dictById(id)
  return d ? `${d.code}（${d.name}）` : `#${id}`
}

/** 接口级例外的宿主接口名：所在 collection 未加载时降级显示 id */
function requestLabel(requestId: number): string {
  const req = requestStore.findRequestById(requestId)
  return req ? `接口「${req.name}」` : `接口 #${requestId}`
}

// ── 解绑 / 删除 ────────────────────────────────────────────
async function removeRule(fieldName: string) {
  const pid = currentProjectId.value
  if (pid == null) return
  try {
    await store.removeFieldRule(pid, fieldName)
    message.success(`已删除项目级规则「${fieldName}」`)
  } catch (e) {
    message.error(String(e))
  }
}

async function removeOverride(o: FieldDictionaryOverride) {
  const pid = currentProjectId.value
  if (pid == null) return
  try {
    await store.removeFieldOverride(pid, o.request_id, o.field_name)
    message.success(`已删除接口级例外「${o.field_name}」`)
  } catch (e) {
    message.error(String(e))
  }
}

// ── 新建项目级规则 ─────────────────────────────────────────
const newField = ref('')
const newDict = ref<number | null>(null)
const canAdd = computed(() => !!newField.value.trim() && newDict.value != null)

async function addRule() {
  const pid = currentProjectId.value
  const field = newField.value.trim()
  if (pid == null || !field || newDict.value == null) {
    message.warning('请填写字段名并选择字典')
    return
  }
  try {
    await store.setFieldRule(pid, field, newDict.value)
    message.success(`已绑定字段「${field}」`)
    newField.value = ''
    newDict.value = null
  } catch (e) {
    message.error(String(e))
  }
}
</script>

<style scoped>
.dict-rules-tab {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
  overflow-y: auto;
  flex: 1;
  min-height: 0;
  height: 100%;
}

.rules-toolbar {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  flex-shrink: 0;
}
.rules-search {
  flex: 1;
  min-width: 0;
}
.rules-dictfilter {
  width: 200px;
  flex-shrink: 0;
}

.rules-filter-hint {
  font-size: var(--font-size-sm);
  color: var(--text-tertiary);
  flex-shrink: 0;
}
.rules-filter-hint code {
  background: var(--bg-hover);
  border-radius: 3px;
  padding: 0 4px;
  font-family: monospace;
  font-size: 12px;
  color: var(--color-primary);
}

.rules-section {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}
.section-title {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  font-size: var(--font-size-sm);
  font-weight: 600;
  color: var(--text-tertiary);
}
.section-hint {
  font-weight: 400;
  color: var(--text-tertiary);
  opacity: 0.8;
}
.section-empty {
  font-size: var(--font-size-sm);
  color: var(--text-tertiary);
  padding: var(--spacing-xs) 0;
}

.rule-row {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  padding: var(--spacing-xs) var(--spacing-sm);
  border: 1px solid var(--border-base);
  border-radius: var(--radius-sm);
  font-size: var(--font-size-sm);
}
.rule-field {
  font-family: monospace;
  color: var(--color-primary);
  font-weight: 600;
  flex-shrink: 0;
}
.rule-arrow {
  color: var(--text-tertiary);
  flex-shrink: 0;
}
.rule-dict {
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  min-width: 0;
}
.rule-dict.is-unbound {
  color: var(--color-warning);
}
.rule-host {
  color: var(--text-tertiary);
  font-size: var(--font-size-sm);
  flex-shrink: 0;
  max-width: 180px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.rules-add {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
  border-top: 1px solid var(--border-base);
  padding-top: var(--spacing-sm);
}
.rules-add-row {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
}
.rules-add-field {
  width: 160px;
  flex-shrink: 0;
}
.rules-add-dict {
  flex: 1;
  min-width: 0;
}
.rules-add-hint {
  font-size: var(--font-size-sm);
  color: var(--text-tertiary);
}
</style>
