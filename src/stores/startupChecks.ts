/**
 * 启动期一次性检查（1.0.6）
 *
 * 统一收编「启动时要做的轻量检查」，一次启动各跑一遍、尽量安静不打扰：
 * - 检查更新（同一**目标版本**只提示 1 次；服务器不可达时静默）
 * - 首启技能引导（技能**完全没装过**且有 agent 目录可用时弹窗；
 *   标记按 App 版本记 —— 同版本不重复，App 升级后若仍完全没装会再提示一次）
 *
 * flag 统一存 `app-settings.json`（经 _persistedSettings，自动落盘，失败不抛）。
 */
import { check } from '@tauri-apps/plugin-updater'
import { getVersion } from '@tauri-apps/api/app'
import { readSetting, writeSetting } from './_persistedSettings'
import { useUpdateStore } from './update'

const KEY_LAST_CHECKED_VERSION = 'startup.lastShownUpdateVersion'
/** 存的是「提示时所在的 App 版本」（字符串），而非布尔 —— 用于 App 升级后重新评估 */
const KEY_SKILL_GUIDE_SHOWN = 'startup.skillGuideShownVersion'

/**
 * 启动时检查更新。
 * - 同一「目标版本」只提示 1 次：记录上次**提示过的新版本号**（如 1.0.7），
 *   下次 check 到的目标版本与之相同则跳过 —— 既不会重复烦，出了更新的版本（1.0.8）
 *   仍会再提示一次（那是新消息）
 * - check() 失败（服务器不可达 / release 为草稿）→ 静默，不打扰用户
 */
export async function checkForUpdateAtStartup(params: {
  /**
   * 返回 true 表示当前已有模态在屏（如技能引导弹窗）。
   * 此时本次不弹更新框、也不写「已提示」标记 —— 让位给已有模态，留到下次启动再提示，
   * 避免两个模态叠加（首启 + 恰好有新版本是真实可复现的场景）。
   */
  isBlocked?: () => boolean
}): Promise<void> {
  const { isBlocked } = params
  try {
    // 先查再比对：必须知道「最新版本是什么」才能判断该版本是否提示过，
    // 因此不能提前 return 省掉这次请求（一次启动一个小 GET，与主流应用做法一致）
    const update = await check()
    if (!update) return // 已是最新，不打扰

    const lastPrompted = await readSetting<string>(KEY_LAST_CHECKED_VERSION)
    if (lastPrompted === update.version) return // 这个目标版本已提示过

    if (isBlocked?.()) return // 已有模态在屏 → 让位，下次启动再说

    // 交给全局弹窗渲染：更新日志是 Markdown，纯文本 dialog 显示不了格式
    useUpdateStore().show(update)
    // 记录「已提示过的目标版本」：同版本不再重复提示，出了更新的版本会再提示
    await writeSetting(KEY_LAST_CHECKED_VERSION, update.version)
  } catch (e) {
    // 更新服务器不可达：静默（前端更新检查不应打断启动）
    console.debug('[startup] 检查更新跳过（无法连接更新服务器）:', e)
  }
}

/** 首启技能引导是否「当前 App 版本已提示过」 */
export async function skillGuideShownThisVersion(): Promise<boolean> {
  const current = await getVersion()
  return (await readSetting<string>(KEY_SKILL_GUIDE_SHOWN)) === current
}

/** 标记首启技能引导已在当前 App 版本提示过（App 升级后若技能仍完全没装，会再提示一次） */
export async function markSkillGuideShown(): Promise<void> {
  await writeSetting(KEY_SKILL_GUIDE_SHOWN, await getVersion())
}