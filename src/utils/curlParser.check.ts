/**
 * parseCurl 自检脚本（零依赖，Node 22+ 可直接跑）：
 *
 *   node --experimental-strip-types src/utils/curlParser.check.ts
 *
 * 只断言会真出错的边界，不做覆盖率表演。
 */
import assert from 'node:assert/strict'
import { parseCurl } from './curlParser.ts'

// ── 根因 #5：-b 此前完全被静默丢弃 ──
{
  const p = parseCurl(`curl 'https://api.example.com/v1/user' -b 'sid=abc; uid=1'`)
  assert.equal(p.cookie, 'sid=abc; uid=1', '-b 的 cookie 必须被解析出来')
  assert.equal(p.headers.some(h => h.key.toLowerCase() === 'cookie'), false, '-b 不该额外造 header')
}

// ── Q11-A：-H 'Cookie: ...' 与 -b 等价，且不产生双头 ──
{
  const p = parseCurl(`curl 'https://a.com/' -H 'Cookie: sid=abc' -b 'uid=1'`)
  assert.equal(p.cookie, 'uid=1; sid=abc', '两条来源合并，-b 在前')
  assert.equal(p.headers.filter(h => h.key.toLowerCase() === 'cookie').length, 0, '不得残留 Cookie header')
}

// ── -b 接 jar 文件路径时不能当 cookie 串写进去 ──
{
  const p = parseCurl(`curl 'https://a.com/' -b /tmp/cookies.txt`)
  assert.equal(p.cookie, '', '文件路径不是 cookie 串')
  assert.ok(p.ignored.some(s => s.includes('cookies.txt')), '必须显式提示被忽略')
}

// ── Q12-A：-u / -A / -e 映射成 header ──
{
  const p = parseCurl(`curl 'https://a.com/' -u 'user:p@ss' -A 'MyAgent/1.0' -e 'https://ref.com/'`)
  const get = (k: string) => p.headers.find(h => h.key.toLowerCase() === k.toLowerCase())?.value
  assert.ok(get('Authorization')?.startsWith('Basic '), '-u 必须转成 Basic')
  assert.equal(get('User-Agent'), 'MyAgent/1.0')
  assert.equal(get('Referer'), 'https://ref.com/')
  // 账号密码含特殊字符时不得抛异常，且解码回来要对得上
  const decoded = Buffer.from(get('Authorization')!.slice(6), 'base64').toString('utf8')
  assert.equal(decoded, 'user:p@ss')
}

// ── Q12-A：-L/-k/--compressed 显式提示，且不得因单别名拼出 `undefined` ──
{
  const p = parseCurl(`curl 'https://a.com/' -L -k --compressed`)
  assert.equal(p.ignored.length, 3, `三个都该提示，实际：${JSON.stringify(p.ignored)}`)
  assert.ok(p.ignored.every(s => !s.includes('undefined')), '别名拼接不得出现 undefined')
  // 回归：单别名 flag 的正则若拼出 (?:--compressed|undefined)，文本里出现 "undefined" 就会误报
  const p2 = parseCurl(`curl 'https://a.com/' --data 'undefined'`)
  assert.equal(p2.ignored.length, 0, 'body 里的 "undefined" 不得被当成 flag')
}

// ── Q7-C：URL 仍只认 http(s)；method 启发式不变 ──
{
  assert.equal(parseCurl(`curl 'ftp://a.com/'`).url, '')
  assert.equal(parseCurl(`curl 'https://a.com/' -d 'x=1'`).method, 'POST')
  assert.equal(parseCurl(`curl 'https://a.com/'`).method, 'GET')
}

// ── body 引号配对：JSON 里含双引号不得截断 ──
{
  const p = parseCurl(`curl 'https://a.com/' -H 'Content-Type: application/json' --data-raw '{"a":"b"}'`)
  assert.equal(p.body, '{"a":"b"}')
  assert.equal(p.bodyType, 'raw_json')
}

console.log('curlParser.check.ts: 全部断言通过')
