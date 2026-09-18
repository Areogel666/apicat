import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { DataDictionary, DictionaryItem, FieldDictionaryRule, FieldDictionaryOverride } from '../types'

/**
 * 数据字典 Store —— 全局共享字典（builtin）与项目自定义字典。
 * items 按 dictionary_id 分桶缓存。
 * 1.0.4「字段名绑定」：项目级 field_name ↔ dictionary 规则 + 接口×字段名例外
 */
export const useDictionaryStore = defineStore('dictionary', () => {
  const dictionaries = ref<DataDictionary[]>([])
  // dictionaryId → DictionaryItem[]
  const itemsMap = ref<Record<number, DictionaryItem[]>>({})
  // 1.0.4：字段名 ↔ 字典 绑定规则 / 例外
  const fieldRules = ref<FieldDictionaryRule[]>([])
  const fieldOverrides = ref<FieldDictionaryOverride[]>([])

  // 1.0.4 fix：字典管理页选中的字典 id（左侧树 → 右侧 JSON 编辑面板联动）
  const selectedDictId = ref<number | null>(null)
  const selectedDict = computed<DataDictionary | null>(() =>
    selectedDictId.value != null
      ? (dictionaries.value.find(d => d.id === selectedDictId.value) ?? null)
      : null,
  )

  function setSelectedDict(id: number | null) {
    selectedDictId.value = id
  }

  // loadSeq 防竞态：启动时 currentProjectId 会被连续改两次（loadProjects → restoreLastProject），
  // 两个常驻组件的 watch 各触发一次加载，后完成者胜 —— 过期响应必须丢弃，
  // 否则旧项目的 rules 残留内存，dictIdForField 会把新项目同名字段误判为已绑定。
  let dictionariesSeq = 0
  let fieldBindingSeq = 0
  // fieldRules/fieldOverrides 实际所属的 project_id（dictIdForField 兜底校验用）
  const fieldBindingsProjectId = ref<number | null>(null)

  async function loadDictionaries(projectId: number | null = null) {
    const seq = ++dictionariesSeq
    const list = await invoke<DataDictionary[]>('list_dictionaries', { projectId })
    // 预加载所有可视字典的 items（字典量小，全量拉取简单）。
    // 并发拉取：N 个字典一次往返 N 条 IPC，而非串行 N 次。
    const itemsList = await Promise.all(
      list.map(d =>
        invoke<DictionaryItem[]>('list_dictionary_items', { dictionaryId: d.id })
      )
    )
    if (seq !== dictionariesSeq) return
    dictionaries.value = list
    // 整表替换而非累加，避免切项目后旧字典 items 残留
    const items: Record<number, DictionaryItem[]> = {}
    list.forEach((d, i) => { items[d.id] = itemsList[i] })
    itemsMap.value = items
  }

  /** 新建数据字典（projectId 传 null 为全局共享，传具体 id 为项目私有） */
  async function createDictionary(code: string, name: string, description = '', projectId: number | null = null) {
    const d = await invoke<DataDictionary>('create_dictionary', {
      code,
      name,
      description,
      projectId,
    })
    dictionaries.value.push(d)
    itemsMap.value[d.id] = []
    return d
  }

  /** 一键新建字典 + 初始字典项（同事务） */
  async function createDictionaryWithItems(
    code: string,
    name: string,
    description: string,
    items: Array<{ label: string; value: string; description?: string }>,
    projectId: number | null = null,
  ) {
    const d = await invoke<DataDictionary>('create_dictionary_with_items', {
      code,
      name,
      description,
      projectId,
      items,
    })
    dictionaries.value.push(d)
    itemsMap.value[d.id] = await invoke<DictionaryItem[]>('list_dictionary_items', {
      dictionaryId: d.id,
    })
    return d
  }

  /** 1.0.4 fix：以 JSON 全量替换某字典的字典项（先清空再插入） */
  async function replaceDictionaryItems(
    dictionaryId: number,
    items: Array<{ label: string; value: string; description?: string }>,
  ) {
    const rows = await invoke<DictionaryItem[]>('replace_dictionary_items', { dictionaryId, items })
    itemsMap.value[dictionaryId] = rows
    return rows
  }

  async function updateDictionary(id: number, data: { name?: string; description?: string }) {
    const d = await invoke<DataDictionary>('update_dictionary', { id, ...data })
    const i = dictionaries.value.findIndex(x => x.id === id)
    if (i >= 0) dictionaries.value[i] = d
    return d
  }

  async function deleteDictionary(id: number) {
    await invoke('delete_dictionary', { id })
    dictionaries.value = dictionaries.value.filter(x => x.id !== id)
    delete itemsMap.value[id]
  }

  async function createItem(dictionaryId: number, label: string, value: string, description = '') {
    const item = await invoke<DictionaryItem>('create_dictionary_item', {
      dictionaryId,
      label,
      value,
      description,
    })
    itemsMap.value[dictionaryId].push(item)
    return item
  }

  async function updateItem(
    id: number,
    data: { label?: string; value?: string; description?: string; sortOrder?: number },
  ) {
    const item = await invoke<DictionaryItem>('update_dictionary_item', { id, ...data })
    for (const list of Object.values(itemsMap.value)) {
      const i = list.findIndex(x => x.id === id)
      if (i >= 0) { list[i] = item; break }
    }
    return item
  }

  async function deleteItem(id: number) {
    await invoke('delete_dictionary_item', { id })
    for (const list of Object.values(itemsMap.value)) {
      const i = list.findIndex(x => x.id === id)
      if (i >= 0) { list.splice(i, 1); break }
    }
  }

  // ── 1.0.4「字段名绑定」：规则 / 例外 / 复制 ────────────────────

  async function loadFieldBindings(projectId: number | null) {
    const seq = ++fieldBindingSeq
    if (projectId == null) {
      fieldRules.value = []
      fieldOverrides.value = []
      fieldBindingsProjectId.value = null
      return
    }
    const [rules, overrides] = await Promise.all([
      invoke<FieldDictionaryRule[]>('list_field_rules', { projectId }),
      invoke<FieldDictionaryOverride[]>('list_field_overrides', { projectId }),
    ])
    if (seq !== fieldBindingSeq) return
    fieldRules.value = rules
    fieldOverrides.value = overrides
    fieldBindingsProjectId.value = projectId
  }

  async function setFieldRule(projectId: number, fieldName: string, dictionaryId: number) {
    await invoke('set_field_rule', { projectId, fieldName, dictionaryId })
    // 本地同步：id 为占位（-1，刷新后由后端补齐）；更新/删除按 project_id+field_name 业务键定位
    upsertLocal(
      fieldRules.value,
      { id: -1, project_id: projectId, field_name: fieldName, dictionary_id: dictionaryId },
      r => r.project_id === projectId && r.field_name === fieldName,
    )
  }

  async function removeFieldRule(projectId: number, fieldName: string) {
    await invoke('delete_field_rule', { projectId, fieldName })
    fieldRules.value = fieldRules.value.filter(r => !(r.project_id === projectId && r.field_name === fieldName))
  }

  async function setFieldOverride(
    projectId: number, requestId: number, fieldName: string, dictionaryId: number | null,
  ) {
    await invoke('set_field_override', { projectId, requestId, fieldName, dictionaryId })
    upsertLocal(
      fieldOverrides.value,
      { id: -1, project_id: projectId, request_id: requestId, field_name: fieldName, dictionary_id: dictionaryId },
      o => o.project_id === projectId && o.request_id === requestId && o.field_name === fieldName,
    )
  }

  async function removeFieldOverride(projectId: number, requestId: number, fieldName: string) {
    await invoke('delete_field_override', { projectId, requestId, fieldName })
    fieldOverrides.value = fieldOverrides.value.filter(o =>
      !(o.project_id === projectId && o.request_id === requestId && o.field_name === fieldName))
  }

  /** 复制字典到目标项目（含字典项 + 源项目里绑定它的字段规则） */
  async function copyDictionaryToProject(
    sourceDictionaryId: number,
    sourceProjectId: number,
    targetProjectId: number,
  ): Promise<DataDictionary> {
    const d = await invoke<DataDictionary>('copy_dictionary_to_project', {
      sourceDictionaryId,
      sourceProjectId,
      targetProjectId,
    })
    dictionaries.value.push(d)
    itemsMap.value[d.id] = await invoke<DictionaryItem[]>('list_dictionary_items', { dictionaryId: d.id })
    // 目标项目规则可能新增了绑定，刷新
    await loadFieldBindings(targetProjectId)
    return d
  }

  /**
   * 解析某字段最终绑定的字典 id：
   * 接口×字段名例外优先（dictionary_id=null 表示解除绑定 → 返回 null 且不回落规则）；
   * 否则用项目规则。
   */
  function dictIdForField(fieldName: string, activeRequestId: number | null | undefined): number | null {
    if (activeRequestId != null) {
      const ov = fieldOverrides.value.find(o => o.request_id === activeRequestId && o.field_name === fieldName)
      if (ov) return ov.dictionary_id
    }
    return ruleDictIdForField(fieldName)
  }

  /** 仅查项目级规则（带 project_id 校验，防 fieldRules 残留他项目数据） */
  function ruleDictIdForField(fieldName: string): number | null {
    const rule = fieldRules.value.find(
      r => r.field_name === fieldName && r.project_id === fieldBindingsProjectId.value,
    )
    return rule ? rule.dictionary_id : null
  }

  /** 本地列表 upsert：命中替换、否则追加（用于规则/例外的本地镜像同步） */
  function upsertLocal<T>(list: T[], item: T, match: (t: T) => boolean) {
    const i = list.findIndex(match)
    if (i >= 0) list[i] = item
    else list.push(item)
  }

  /** 按 id 取字典 */
  function dictById(id: number | null | undefined): DataDictionary | null {
    if (id == null) return null
    return dictionaries.value.find(d => d.id === id) ?? null
  }

  return {
    dictionaries,
    itemsMap,
    fieldRules,
    fieldOverrides,
    selectedDictId,
    selectedDict,
    setSelectedDict,
    loadDictionaries,
    loadFieldBindings,
    setFieldRule,
    removeFieldRule,
    setFieldOverride,
    removeFieldOverride,
    copyDictionaryToProject,
    dictIdForField,
    ruleDictIdForField,
    dictById,
    createDictionary,
    createDictionaryWithItems,
    updateDictionary,
    deleteDictionary,
    createItem,
    updateItem,
    deleteItem,
    replaceDictionaryItems,
  }
})