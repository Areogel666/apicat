<script setup lang="ts">
import { reactive, h, defineComponent, watch } from 'vue'
import type { VNode } from 'vue'
import type { DictMarker } from './jsonTreeUtils'

/**
 * 自研 JSON 树渲染（替换 vue-json-pretty）
 *
 * 替换原因：vue-json-pretty 不开放字段级自定义渲染，无法在字段值旁挂字典命中微标签。
 * 能力对齐（不退步）：折叠/展开、初始展开深度、行号（CSS counter）、长度统计。
 *
 * 命中微标签：decorate(key, value) 返回 DictMarker 时，字段值后渲染 `▸ label` 小标签，
 * hover（原生 title）展示字典名 + item 描述。
 */

interface Props {
  data: unknown
  /** 初始展开深度（默认 3，与 vue-json-pretty 行为一致；Ctrl+F 全展开传 999） */
  deep?: number
  /** 字段命中判定：返回 null=不渲染标记 */
  decorate?: (key: string, value: unknown) => DictMarker | null
}

const props = withDefaults(defineProps<Props>(), { deep: 3, decorate: undefined })

const JsonTree = defineComponent({
  name: 'JsonTree',
  props: {
    node: { type: null, default: null },
    depth: { type: Number, default: 0 },
    path: { type: String, default: '' },
    keyName: { type: String, default: null },
    deepInit: { type: Number, default: 3 },
    decorate: { type: Function, default: null },
  },
  setup(props) {
    // 显式展开状态：path -> 是否展开（用户点击产生的，优先级高于默认深度）
    //
    // 为什么不能只用「collapsedPaths + depth < deepInit」：
    //   expanded = depth < deepInit && !collapsed 里，depth < deepInit 对深于默认层级的节点
    //   恒为 false —— 此时 toggle() 改 collapsedPaths 也不会改变结果，表现为「点击无法展开」，
    //   只有 Ctrl+F 把 deepInit 顶到 999 后才恢复正常。故必须让显式状态优先、默认深度兜底。
    const explicitExpanded = reactive(new Map<string, boolean>())

    /** 当前是否展开：显式状态优先，否则按默认深度 */
    function isExpanded(path: string, depth: number): boolean {
      const v = explicitExpanded.get(path)
      return v !== undefined ? v : depth < props.deepInit
    }

    // deepInit 变化（Ctrl+F 全展开 / 退出搜索回到默认）时清空显式状态，回到默认视图
    watch(() => props.deepInit, () => explicitExpanded.clear())

    function isObj(v: unknown): boolean {
      return v !== null && typeof v === 'object' && !Array.isArray(v)
    }
    function isArr(v: unknown): boolean {
      return Array.isArray(v)
    }
    function countOf(v: unknown): number {
      if (isArr(v)) return (v as unknown[]).length
      if (isObj(v)) return Object.keys(v as Record<string, unknown>).length
      return 0
    }

    function scalarText(value: unknown): string {
      if (value === null) return 'null'
      if (typeof value === 'string') return JSON.stringify(value)
      return String(value)
    }
    function scalarClass(value: unknown): string {
      if (typeof value === 'string') return 'jt-string'
      if (typeof value === 'number') return 'jt-number'
      if (typeof value === 'boolean') return 'jt-boolean'
      return 'jt-null'
    }

    /** 整行点击 = 折叠/展开切换（含 bracket）：记录与该节点当前状态相反的显式状态 */
    function toggle(path: string, depth: number) {
      explicitExpanded.set(path, !isExpanded(path, depth))
    }

    function renderNode(value: unknown, key: string | null, path: string, depth: number): VNode {
      const prefix = (): VNode => (
        key != null
          ? h('span', { class: 'jt-pfx' }, [
              h('span', { class: 'jt-key' }, key),
              h('span', { class: 'jt-punct' }, ': '),
            ])
          : h('span', { class: 'jt-pfx' })
      )

      if (!isObj(value) && !isArr(value)) {
        // 标量叶子：值 + 可选字典命中微标签
        const segs: (string | VNode)[] = [prefix(), h('span', { class: ['jt-val', scalarClass(value)] }, scalarText(value))]
        const marker = key != null && props.decorate ? (props.decorate as (k: string, v: unknown) => DictMarker | null)(key, value) : null
        if (marker) {
          segs.push(h('span', { class: 'jt-mark', title: marker.tooltip }, ` ▸ ${marker.label}`))
        }
        return h('div', { class: 'jt-line' }, segs)
      }

      const bracketOpen = isArr(value) ? '[' : '{'
      const bracketClose = isArr(value) ? ']' : '}'
      const expanded = isExpanded(path, depth)

      // 折叠态：`{ … N }` 一行
      if (!expanded) {
        return h('div', { class: 'jt-line jt-collapsed', onClick: () => toggle(path, depth) }, [
          prefix(),
          h('span', { class: 'jt-bracket' }, bracketOpen),
          h('span', { class: 'jt-ellipsis' }, `… ${countOf(value)} 项 `),
          h('span', { class: 'jt-bracket' }, bracketClose),
        ])
      }

      // 展开态：`{` / children / `}`
      const children: VNode[] = []
      if (isObj(value)) {
        for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
          children.push(renderNode(v, k, `${path}.${k}`, depth + 1))
        }
      } else {
        ;(value as unknown[]).forEach((v, i) => {
          children.push(renderNode(v, null, `${path}[${i}]`, depth + 1))
        })
      }

      return h('div', { class: 'jt-collapsible' }, [
        h('div', { class: 'jt-line jt-line-open', onClick: () => toggle(path, depth) }, [
          prefix(),
          h('span', { class: 'jt-bracket' }, bracketOpen),
        ]),
        h('div', { class: 'jt-children' }, children),
        h('div', { class: 'jt-line jt-line-close', onClick: () => toggle(path, depth) }, [
          h('span', { class: 'jt-bracket' }, bracketClose),
        ]),
      ])
    }

    return () => h('div', { class: 'json-tree jt-show-line' }, [renderNode(props.node, props.keyName, props.path, props.depth)])
  },
})
</script>

<template>
  <JsonTree :node="props.data" :deep-init="props.deep" :decorate="props.decorate" />
</template>

<style>
/* 注意：必须非 scoped —— 树内容由 h() 动态生成 vnode，scoped 选择器不会作用到
   这些节点（scope 属性只加在模板书写的元素上），会导致缩进/行号/颜色全部失效。
   .jt-* 类名足够唯一，无跨组件冲突。 */
.json-tree {
  counter-reset: jtLine;
  color: var(--text-primary);
  word-break: break-all;
  white-space: pre-wrap;
}
/* 行号（CSS counter：折叠/展开自动重算） */
.jt-line {
  counter-increment: jtLine;
  padding-left: 34px;
  position: relative;
  border-radius: 2px;
  cursor: default;
}
.jt-collapsed,
.jt-line-open,
.jt-line-close {
  cursor: pointer;
}
.jt-line:hover {
  background: var(--bg-hover);
}
.jt-line::before {
  content: counter(jtLine);
  position: absolute;
  left: 8px;
  width: 20px;
  text-align: right;
  color: var(--text-tertiary);
  opacity: 0.6;
  font-size: 11px;
  user-select: none;
}
.jt-pfx {
  user-select: none;
}
.jt-key {
  color: var(--json-key);
}
.jt-punct {
  color: var(--text-tertiary);
}
.jt-bracket {
  color: var(--text-tertiary);
  cursor: pointer;
}
.jt-ellipsis {
  color: var(--text-tertiary);
  cursor: pointer;
}
.jt-val {
  user-select: text;
}
.jt-string { color: var(--json-string); }
.jt-number { color: var(--json-number); }
.jt-boolean { color: var(--json-boolean); }
.jt-null { color: var(--json-null); font-style: italic; }
.jt-children {
  padding-left: 16px;
}
/* 字典命中微标签：主色系胶囊标签，与响应体普通文本明显区分（一眼可辨是「标注」） */
.jt-mark {
  margin-left: 6px;
  padding: 0 5px;
  font-size: 11px;
  line-height: 1.6;
  border-radius: 999px;
  background: var(--color-primary-soft);
  color: var(--color-primary);
  font-weight: 600;
  white-space: nowrap;
  cursor: help;
  user-select: text;
}
</style>