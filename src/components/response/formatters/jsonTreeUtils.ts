/**
 * JSON 树 + 响应字典命中 工具集
 *
 * looseEq 从 FieldDictDesc.vue 抽离（行为零变化），供两处复用；
 * matchDictMarker 是「字段值 → 字典命中标记」的纯判定，JsonRenderer 注入 store 闭包调用。
 */

/** 数字与字符串互通：0 === "0"；空串/空白不参与数值比较，避免空值误命中 value=0 */
export function looseEq(a: string, b: string): boolean {
  if (a == null || b == null) return a === b
  const ta = a.trim()
  const tb = b.trim()
  if (ta === tb) return true
  if (ta === '' || tb === '') return false
  const na = Number(ta)
  const nb = Number(tb)
  return Number.isFinite(na) && Number.isFinite(nb) && na === nb
}

/** 字典命中标记：JSON 树中字段值旁要渲染的微标签内容 */
export interface DictMarker {
  /** 微标签文案（如 `1 ▸ 审核中`） */
  label: string
  /** hover 提示（可含换行：字典名 / item 描述） */
  tooltip: string
}

/** 字典项的最小结构（JsonRenderer 注入 itemsMap 的对象即满足） */
export interface DictMarkerItem {
  value: string
  label: string
  description?: string | null
}

/**
 * 在字典里找字段值命中项；未绑定字典或未命中返回 null（不渲染标记）。
 * 约定：仅按字段名精确匹配（第一版不做嵌套路径）。
 *
 * @param dictIdForField  字段 → 绑定字典 id（null=未绑定），复用现有规则表
 * @param dictName        字典 id → 展示名（tooltip 首行）
 * @param itemList        字典 id → 字典项列表
 * @param key             字段名（JSON 对象属性名）
 * @param value           字段值（仅标量；对象/数组不参与）
 */
export function matchDictMarker(
  dictIdForField: (key: string) => number | null,
  dictName: (id: number) => string,
  itemList: (id: number) => DictMarkerItem[],
  key: string,
  value: unknown,
): DictMarker | null {
  const dictId = dictIdForField(key)
  if (dictId == null) return null
  const items = itemList(dictId)
  const str = String(value)
  const it = items.find(i => looseEq(str, i.value))
  if (!it) return null
  const tip = [dictName(dictId), `${it.value} = ${it.label}`, it.description ?? ''].filter(Boolean).join('\n')
  return { label: it.label, tooltip: tip }
}