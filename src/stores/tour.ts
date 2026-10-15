/**
 * 首启功能导览（driver.js）的跨组件协调信号。
 * AppLayout 持有 driver 实例与 leftPanel，StartupChecks / TopBar 是触发方——
 * 通过 pendingStart 通知 AppLayout 启动 tour；finishNotify 在 tour 结束（含中断）时回调，
 * 供 StartupChecks 阻塞等待 tour 完成后再弹技能引导。
 *
 * finishNotify 用集合：tour 进行中再触发「重看新手引导」（requestStart 无回调）不会
 * 覆盖丢失启动链的等待回调。
 */
import { ref } from 'vue'
import { defineStore } from 'pinia'

export const useTourStore = defineStore('tour', () => {
  const pendingStart = ref(false)
  const finishNotify = ref<Set<() => void>>(new Set())

  /** 请求启动 tour；onFinish 可选——tour 结束（完成或中断）时回调一次 */
  function requestStart(onFinish?: () => void) {
    if (onFinish) finishNotify.value.add(onFinish)
    pendingStart.value = true
  }

  /** AppLayout 消费启动信号 */
  function consumePending() {
    pendingStart.value = false
  }

  /** AppLayout 在 tour 结束时调用（无论完成还是中断） */
  function notifyFinished() {
    const cbs = [...finishNotify.value]
    finishNotify.value.clear()
    for (const cb of cbs) cb()
  }

  return { pendingStart, requestStart, consumePending, notifyFinished }
})
