import { ref, watch } from 'vue'
import { useProjectStore } from '../stores/project'

/**
 * 侧栏树展开状态持久化：按「树 × 项目」存 localStorage。
 * key：`tree.expanded.{name}:{projectId}`，value：节点 key 数组的 JSON。
 *
 * expandedKeys 即受控展开源（配合 n-tree 的 v-model:expanded-keys），
 * 调用方在数据加载完成后调 restore() 恢复持久化值，无记录时自行给默认值。
 * 用户每次展开/收起都会实时写盘，切项目/重启均不丢。
 */
export function useTreeExpanded(name: string) {
  const projectStore = useProjectStore()
  const expandedKeys = ref<string[]>([])

  function keyFor(pid: number | null): string | null {
    return pid == null ? null : `tree.expanded.${name}:${pid}`
  }

  /** 读取指定项目（缺省当前项目）的持久化展开状态；无记录或解析失败返回 null */
  function restore(pid?: number | null): string[] | null {
    const key = keyFor(pid ?? projectStore.currentProjectId)
    if (!key) return null
    try {
      const raw = localStorage.getItem(key)
      const arr = raw ? JSON.parse(raw) : null
      return Array.isArray(arr) ? (arr as string[]) : null
    } catch {
      return null
    }
  }

  // 展开变化即写盘；未选项目时无归属，跳过
  watch(expandedKeys, (keys) => {
    const key = keyFor(projectStore.currentProjectId)
    if (!key) return
    try {
      localStorage.setItem(key, JSON.stringify(keys))
    } catch {
      // 写入失败（容量/隐私模式）不影响功能
    }
  }, { deep: true })

  return { expandedKeys, restore }
}
