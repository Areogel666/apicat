/**
 * 新手指引状态：每个引导一个 seen 标记，持久化到 app-settings.json 的 guide.* 命名空间。
 * 口径：键不存在 = 未看过（老用户升级后各引导触发一次）。
 *
 * 1.0.9：面板级 GuideTip（dictionary/stats）已由首启 tour 承担并移除，
 * 保留的都是 tour 覆盖不到的功能内上下文提示（GuideBar × 5 + firstRequest）。
 */
import { reactive } from 'vue'
import { defineStore } from 'pinia'
import { readSetting, writeSetting, deleteSetting } from './_persistedSettings'

export type GuideId =
  | 'stress' | 'cookie' | 'skills' | 'import' | 'env' | 'firstRequest' | 'tour'

export const GUIDE_IDS: readonly GuideId[] = [
  'stress', 'cookie', 'skills', 'import', 'env', 'firstRequest', 'tour',
]

const keyOf = (id: GuideId) => `guide.${id}`

export const useGuideStore = defineStore('guide', () => {
  const seen = reactive<Record<GuideId, boolean>>({
    stress: false, cookie: false, skills: false,
    import: false, env: false, firstRequest: false, tour: false,
  })

  /** 启动时从 app-settings.json 载入各 seen 标记。读失败视为未看过（try/catch 在 readSetting 内）。 */
  async function init(): Promise<void> {
    await Promise.all(
      GUIDE_IDS.map(async (id) => {
        const v = await readSetting<boolean>(keyOf(id))
        seen[id] = v === true
      }),
    )
  }

  function isSeen(id: GuideId): boolean {
    return seen[id]
  }

  function markSeen(id: GuideId): void {
    if (seen[id]) return
    seen[id] = true
    void writeSetting(keyOf(id), true)
  }

  /** 重置全部引导（⚙️「重看新手引导」）。响应式置 false 后，当前 active 的 popover 会立即重现。 */
  function resetAll(): void {
    for (const id of GUIDE_IDS) {
      seen[id] = false
      void deleteSetting(keyOf(id))
    }
  }

  return { seen, init, isSeen, markSeen, resetAll }
})
