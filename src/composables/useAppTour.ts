/**
 * 首启功能导览（1.0.9）：driver.js 聚光灯，5 步走完 左侧四个面板 + ⚙️ 菜单。
 *
 * 实现要点：不用 driver 的多步 drive 模式——面板切换是异步的（leftPanel ref → v-show patch），
 * driver 内部步进是同步的，会踩 DOM 未更新的坑。这里用 highlight 单步模式自驱状态机：
 * 每步先切面板、等 DOM 就绪，再 highlight；下一步/上一步/关闭全部走自己的回调。
 */
import { ref, nextTick, type Ref } from 'vue'
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
  const tourActive = ref(false)
  let driverObj: Driver | null = null

  async function showStep(i: number) {
    if (!driverObj) return
    const step = TOUR_STEPS[i]
    if (step.panel) {
      leftPanel.value = step.panel
      await nextTick()
      // v-show patch 后再等一帧，确保 getBoundingClientRect 拿到真实布局
      await new Promise<void>(r => requestAnimationFrame(() => r()))
    }
    const isLast = i === TOUR_STEPS.length - 1
    driverObj.highlight({
      element: step.element,
      popover: {
        title: step.title,
        description: step.body,
        showButtons: ['close', 'previous', 'next'],
        showProgress: true,
        prevBtnText: '上一步',
        nextBtnText: isLast ? '完成' : '下一步',
        onNextClick: () => (isLast ? finish() : void showStep(i + 1)),
        onPrevClick: () => void showStep(i - 1),
      } as DriveStep['popover'],
    })
  }

  /** tour 结束（完成或中途关闭）。一律记 seen（应用生涯一次性），并通知等待方继续。 */
  function finish() {
    driverObj?.destroy()
    driverObj = null
    tourActive.value = false
    guideStore.markSeen('tour')
    tourStore.notifyFinished()
  }

  function startTour() {
    if (driverObj) return
    tourStore.consumePending()
    tourActive.value = true
    driverObj = driver({
      allowClose: true,
      popoverClass: 'apicat-tour-popover',
      onCloseClick: () => finish(),
    })
    void showStep(0)
  }

  return { tourActive, startTour }
}
