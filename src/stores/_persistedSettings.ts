/**
 * 用户偏好持久化辅助层（M3 引入）
 *
 * 统一封装 @tauri-apps/plugin-store，所有用户级偏好（如上次打开的项目、主题模式）
 * 都存到同一个 store 文件 `app-settings.json`，避免多文件碎片化。
 *
 * 设计要点：
 * - load() 默认 autoSave=true（debounce 100ms 自动落盘），无需显式传
 * - StoreOptions.defaults 是当前 plugin-store 版本的必填字段，传空对象即可
 *   （已存在的 key 仍会从磁盘加载，不会被 defaults 覆盖）
 * - 全部 try/catch 兜底：plugin-store IPC 失败不能阻断业务流程
 *   （参考 stores/AGENTS.md 与 layout/AGENTS.md 的 saveState/restoreState 约定）
 * - 单例 store 实例：load() 是异步的，缓存避免重复 IPC
 *
 * 已声明权限：capabilities/default.json → "store:default"
 *
 * 命名空间约定（key 命名）：
 *   lastOpenedProjectId   M3-A
 *   theme.mode            M3-B
 *   layout.*              预留
 */
import { load, type Store } from '@tauri-apps/plugin-store'

const STORE_FILE = 'app-settings.json'
let _store: Store | null = null

async function getStore(): Promise<Store> {
  if (!_store) {
    _store = await load(STORE_FILE, { defaults: {} })
  }
  return _store
}

/** 读取一个偏好值。读失败 / 不存在均返回 null，不抛异常。 */
export async function readSetting<T>(key: string): Promise<T | null> {
  try {
    const s = await getStore()
    const v = await s.get<T>(key)
    return v ?? null
  } catch (e) {
    console.warn('[settings] read fail:', key, e)
    return null
  }
}

/** 写入一个偏好值。autoSave 模式下自动落盘，写失败仅 warn 不抛。 */
export async function writeSetting<T>(key: string, value: T): Promise<void> {
  try {
    const s = await getStore()
    await s.set(key, value)
  } catch (e) {
    console.warn('[settings] write fail:', key, e)
  }
}

/** 删除一个偏好值（清理脏数据用）。 */
export async function deleteSetting(key: string): Promise<void> {
  try {
    const s = await getStore()
    await s.delete(key)
  } catch (e) {
    console.warn('[settings] delete fail:', key, e)
  }
}
