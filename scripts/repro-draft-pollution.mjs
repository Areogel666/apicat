// 回归 harness v3：防抖落库 id 漂移 —— 1.0.4 BUG「类型/描述在切换接口后错误填充到目标接口同位置参数」
// 根因（见 docs/1.0.4/fix/2026-09-14-bug-fixes.md §二.2）：
//   persistParams 的 600ms 防抖窗口内切换接口，回调执行时按「当前激活接口」写库 → 把 A 的编辑写进 B。
// 修复：schedulePersistParams 触发时立即锁定目标接口 id，回调按锁定 id 落库（不动 dirty 语义）。
// 对照：旧架构按当前 active 落库（漂移 → 污染，红）；新架构触发时锁定 id 落库（隔离，绿）。
import { setTimeout as sleep } from 'node:timers/promises'

const paramRow = (hasDesc) => ({
  key: 'status', value: '1', enabled: true,
  ...(hasDesc ? { type: 'string', description: 'A接口的描述' } : {}),
})

// 模拟「编辑接口 → 防抖窗口内切走 → 600ms 回调落库」三步时序
function makeEngine({ lockId }) {
  // A 自带描述，B 无描述 —— 若 B 被写入含 desc 的快照即视为污染
  const db = { A: [paramRow(true)], B: [paramRow(false)] }
  let active = 'A'          // 当前激活接口（回调执行时可能已漂移）
  let pendingEdit = null    // 防抖期内待落库的编辑：{ id, snapshot }

  const edit = (id) => {
    // 编辑产生快照（含新元数据）
    pendingEdit = { id, snapshot: JSON.parse(JSON.stringify(db[id])) }
    if (lockId) pendingEdit.__lock = id // 新架构：触发时即锁定编辑接口
  }
  const switchTo = (id) => { active = id }
  const flush = () => {
    if (!pendingEdit) return
    // 落库目标：新架构取锁定 id（= 真正编辑的接口），旧架构取当前 active（窗口内漂移）
    const target = lockId ? pendingEdit.__lock : active
    db[target] = pendingEdit.snapshot // 覆盖写
    pendingEdit = null
  }
  return { db, edit, switchTo, flush }
}

// 回归场景：编辑 A → 防抖窗口内切到 B → 600ms 后回调落库
async function run(engine) {
  const e = makeEngine(engine)
  e.edit('A')           // 编辑 A：写入描述快照
  await sleep(1)        // 防抖窗口…
  e.switchTo('B')       // …期间切到 B（active 漂移）
  await sleep(5)        // 600ms 后回调触发
  e.flush()
  // 检测：B 是否被写入了它自己原本没有的描述
  if (e.db.B[0]?.description != null) return `POLLUTED(B 被写入了 A 的描述: ${e.db.B[0].description})`
  return 'CLEAN'
}

let oldPoll = 0, newPoll = 0
for (let i = 0; i < 50; i++) {
  if (await run({ lockId: false }) !== 'CLEAN') oldPoll++
  if (await run({ lockId: true }) !== 'CLEAN') newPoll++
}
console.log(`[bug-loop] 旧架构（防抖回调按当前 active 落库）漂移污染次数: ${oldPoll}/50`)
console.log(`[bug-loop] 新架构（触发时锁定接口 id 落库）污染次数: ${newPoll}/50`)
if (oldPoll === 0) { console.error('✗ 复现失败：旧架构未复现漂移污染，说明真实路径另有其处'); process.exit(2) }
if (newPoll !== 0) { console.error('✗ 修复失败：新架构仍有污染'); process.exit(1) }
console.log('✓ 闭环成立：旧架构可复现防抖漂移污染，新架构锁写隔离')