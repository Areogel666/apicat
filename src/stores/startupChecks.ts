/**
 * 启动期一次性检查（1.0.6）
 *
 * 统一收编「启动时要做的轻量检查」，一次启动各跑一遍、尽量安静不打扰：
 * - 检查更新（同一版本只提示 1 次，服务器不可达时静默）
 * - 首启技能引导（检测到 agent 目录但技能未装时弹窗，提示过不再打扰）
 *
 * flag 统一存 `app-settings.json`（经 _persistedSettings，自动落盘，失败不抛）。
 */
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { getVersion } from '@tauri-apps/api/app'
import { useDialog, useMessage } from 'naive-ui'
import { readSetting, writeSetting } from './_persistedSettings'

type DialogLike = ReturnType<typeof useDialog>
type MessageLike = ReturnType<typeof useMessage>

const KEY_LAST_CHECKED_VERSION = 'startup.lastShownUpdateVersion'
const KEY_SKILL_GUIDE_SHOWN = 'startup.skillGuideShown'

/**
 * 启动时检查更新。
 * - 同一版本只提示 1 次：记录「上次已提示过的新版本号」，本轮当前版本与之相同则跳过
 * - check() 失败（服务器不可达 / release 为草稿）→ 静默，不打扰用户
 * - 只有 handler 提供 dialog/message（手动入口传入；启动流程传入后弹窗即可复用同一逻辑）
 */
export async function checkForUpdateAtStartup(params: {
  dialog: DialogLike
  message: MessageLike
  /**
   * 返回 true 表示当前已有模态在屏（如技能引导弹窗）。
   * 此时本次不弹更新框、也不写「已提示」标记 —— 让位给已有模态，留到下次启动再提示，
   * 避免两个模态叠加（首启 + 恰好有新版本是真实可复现的场景）。
   */
  isBlocked?: () => boolean
}): Promise<void> {
  const { dialog, message, isBlocked } = params
  try {
    const currentVersion = await getVersion()
    const lastShown = await readSetting<string>(KEY_LAST_CHECKED_VERSION)
    if (lastShown === currentVersion) return // 同一版本已提示过

    const update = await check()
    if (!update) return // 已是最新，不打扰

    if (isBlocked?.()) return // 已有模态在屏 → 让位，下次启动再说

    dialog.info({
      title: `发现新版本 ${update.version}`,
      content: update.body
        ? `更新内容：\n${update.body}`
        : '有新版本可用，是否立即更新并重启？',
      positiveText: '立即更新',
      negativeText: '稍后再说',
      onPositiveClick: async () => {
        const downloadMsg = message.loading('正在下载更新，请稍候...', { duration: 0 })
        try {
          await update.downloadAndInstall()
          downloadMsg.destroy()
          message.success('更新完成，正在重启...')
          setTimeout(() => relaunch(), 1500)
        } catch (e) {
          downloadMsg.destroy()
          message.error(`更新失败：${e}`)
        }
      },
    })
    // 无论点没点「立即更新」，都算已提示过该版本，避免每天/每次启动重复弹
    await writeSetting(KEY_LAST_CHECKED_VERSION, currentVersion)
  } catch (e) {
    // 更新服务器不可达：静默（前端更新检查不应打断启动）
    console.debug('[startup] 检查更新跳过（无法连接更新服务器）:', e)
  }
}

/** 首启技能引导是否已提示过 */
export async function skillGuideShown(): Promise<boolean> {
  return (await readSetting<boolean>(KEY_SKILL_GUIDE_SHOWN)) === true
}

/** 标记首启技能引导已提示过 */
export async function markSkillGuideShown(): Promise<void> {
  await writeSetting(KEY_SKILL_GUIDE_SHOWN, true)
}