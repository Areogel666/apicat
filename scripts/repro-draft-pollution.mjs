// 回归 harness v2：接口切换存在异步 watch 回调交错（async + await），
// 检测「类型/描述」是否从接口 A 污染到接口 B。
// 对照：无 token 防护（旧架构）应能复现污染（红）；有 token 防护（新架构）应清零（绿）。
import { setTimeout as sleep } from 'node:timers/promises'

const paramRow = (hasDesc) => ({
  key: 'status', value: '1', enabled: true,
  ...(hasDesc ? { type: 'string', description: 'A接口的描述' } : {}),
})

function makeEngine({ tokenGuard, deepCopy }) {
  const db = { A: [paramRow(true)], B: [paramRow(false)] }
  let editor = db.A
  let active = 'A'
  const draftCache = {}
  let seq = 0
  let curSeq = 0

  async function switchTo(id) {
    const my = ++seq
    // 保存当前接口草稿（异步 watch 的第一段：保存 oldReq 草稿）
    draftCache[active] = { queryParams: deepCopy ? JSON.parse(JSON.stringify(editor ?? [])) : [...editor] }
    // 模拟内部 async：loadHistory / loadTestCases 的 await，长度抖动制造交错
    await sleep(1 + Math.floor(Math.random() * 4))
    // token 防护：过期回调（不是最新一次切换）放弃恢复，避免旧回调把错配编辑区写回
    if (tokenGuard && my !== curSeq) return
    curSeq = my
    const d = draftCache[id]
    editor = d ? d.queryParams : JSON.parse(JSON.stringify(db[id]))
    active = id
  }
  return { switchTo, draftCache, db }
}

// 并发来回切（模拟快速点 Tab 时多个 watch 回调重叠）
async function run(engine) {
  const e = makeEngine(engine)
  const batch = async () => {
    const p1 = e.switchTo('B')
    const p2 = e.switchTo('A')
    const p3 = e.switchTo('B')
    await Promise.all([p1, p2, p3])
  }
  for (let i = 0; i < 3; i++) await batch()
  // 检查任一接口草稿里是否混入了它自己 DB 中没有的描述
  for (const [id, d] of Object.entries(e.draftCache)) {
    for (const p of d.queryParams ?? []) {
      if (p.description && e.db[id][0].description == null) {
        return `POLLUTED(${id} 拿到了不该有的描述: ${p.description})`
      }
    }
  }
  return 'CLEAN'
}

let oldPoll = 0, newPoll = 0
for (let i = 0; i < 50; i++) {
  if (await run({ tokenGuard: false, deepCopy: false }) !== 'CLEAN') oldPoll++
  if (await run({ tokenGuard: true,  deepCopy: true  }) !== 'CLEAN') newPoll++
}
console.log(`[bug-loop] 旧架构（无 token + 浅拷贝）复现污染次数: ${oldPoll}/50`)
console.log(`[bug-loop] 新架构（token + 深拷贝）污染次数: ${newPoll}/50`)
if (oldPoll === 0) { console.error('✗ 复现失败：并发也未触发污染，说明真实路径另有其处'); process.exit(2) }
if (newPoll !== 0) { console.error('✗ 修复失败：新架构仍有污染'); process.exit(1) }
console.log('✓ 闭环成立：旧架构可复现污染，新架构彻底隔离')
