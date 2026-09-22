<script setup lang="ts">
import { reactive, h, defineComponent } from 'vue'
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
    // reactive Set：render 中 has() 建立依赖，toggle 增删自动触发重渲染
    const collapsedPaths = reactive(new Set<string>())

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

    /** 整行点击 = 折叠/展开切换（含 bracket） */
    function toggle(path: string) {
      if (collapsedPaths.has(path)) collapsedPaths.delete(path)
      else collapsedPaths.add(path)
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
      const expanded = depth < props.deepInit && !collapsedPaths.has(path)

      // 折叠态：`{ … N }` 一行
      if (!expanded) {
        return h('div', { class: 'jt-line jt-collapsed', onClick: () => toggle(path) }, [
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
        h('div', { class: 'jt-line jt-line-open', onClick: () => toggle(path) }, [
          prefix(),
          h('span', { class: 'jt-bracket' }, bracketOpen),
        ]),
        h('div', { class: 'jt-children' }, children),
        h('div', { class: 'jt-line jt-line-close', onClick: () => toggle(path) }, [
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

<style scoped>
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
/* 字典命中微标签：值旁小型字典色标签，原生 title 悬停展示全信息 */
.jt-mark {
  margin-left: 4px;
  padding: 0 4px;
  font-size: 11px;
  line-height: 1.5;
  border-radius: 3px;
  background: var(--color-primary-soft, rgba(37, 124, 245, 0.15));
  color: var(--color-primary);
  white-space: nowrap;
  cursor: help;
  user-select: text;
}
</style>