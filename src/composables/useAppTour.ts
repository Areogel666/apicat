/**
 * 首启功能导览（1.0.9）：driver.js 聚光灯，5 步走完 左侧四个面板 + ⚙️ 菜单。
 *
 * 实现要点：
 * - 不用 driver 的多步 drive 模式——面板切换是异步的（leftPanel ref → v-show patch），
 *   driver 内部步进是同步的，会踩 DOM 未更新的坑。这里用 highlight 单步模式自驱状态机：
 *   每步先切面板、等 DOM 就绪，再 highlight；下一步/上一步/关闭全部走自己的回调。
 * - 关闭路径统一走 onDestroyStarted：driver 1.9 里 X 按钮、Esc、点遮罩三条路径都会先
 *   经它（onCloseClick 只覆盖 X），finish() 用标志位防 destroy 重入。
 * - highlight 前的 await 期间 tour 可能已被关闭，恢复执行后必须重新判空。
 */
import { nextTick, type Ref } from 'vue'
import { driver, type Driver, type DriveStep } from 'driver.js'
import 'driver.js/dist/driver.css'
import '../styles/tour.css'
import { TOUR_STEPS } from '../config/guideContent'
import { useGuideStore } from '../stores/guide'
import { useTourStore } from '../stores/tour'

type LeftPanel = 'interface' | 'dictionary' | 'docs' | 'stats'

export function useAppTour(leftPanel: Ref<LeftPanel>) {
  const guideStore = useGuideStore()
  const tourStore = useTourStore()
  let driverObj: Driver | null = null
  let finishing = false

  async function showStep(i: number) {
    if (i < 0 || i >= TOUR_STEPS.length) return
    const step = TOUR_STEPS[i]
    if (step.panel) {
      leftPanel.value = step.panel
      await nextTick()
      // v-show patch 后再等一帧，确保 getBoundingClientRect 拿到真实布局
      await new Promise<void>(r => requestAnimationFrame(() => r()))
    }
    // 两个 await 期间 tour 可能已被关闭（Esc/X/完成）
    if (!driverObj) return
    const isLast = i === TOUR_STEPS.length - 1
    const showPrev = i > 0
    driverObj.highlight({
      element: step.element,
      popover: {
        title: `${step.title}（${i + 1}/${TOUR_STEPS.length}）`,
        description: step.body,
        // highlight 模式不自动禁用/填充按钮（showProgress 也只在 drive 模式有值），进度拼进标题
        showButtons: showPrev ? ['close', 'previous', 'next'] : ['close', 'next'],
        prevBtnText: '上一步',
        nextBtnText: isLast ? '完成' : '下一步',
        onNextClick: () => (isLast ? finish() : void showStep(i + 1)),
        onPrevClick: () => void showStep(i - 1),
      } as DriveStep['popover'],
    })
  }

  /** tour 结束（完成或任何方式的中途关闭）。一律记 seen（应用生涯一次性），并通知等待方继续。 */
  function finish() {
    if (finishing) return
    finishing = true
    driverObj?.destroy()
    driverObj = null
    finishing = false
    guideStore.markSeen('tour')
    tourStore.notifyFinished()
  }

  function startTour() {
    tourStore.consumePending()
    if (driverObj) {
      // 已在跑（如 tour 进行中点「重看新手引导」）：跳回第一步从头再来
      void showStep(0)
      return
    }
    driverObj = driver({
      allowClose: true,
      popoverClass: 'apicat-tour-popover',
      // 覆盖 X / Esc / 遮罩点击所有主动关闭路径；「完成」按钮经 onNextClick → finish 同样置位
      onDestroyStarted: () => finish(),
    })
    void showStep(0)
  }

  return { startTour }
}
