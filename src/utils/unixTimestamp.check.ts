/**
 * Unix 时间戳识别自检（零依赖，Node 22+）：
 *   node --experimental-strip-types src/utils/unixTimestamp.check.ts
 */
import assert from 'node:assert/strict'
import { unixTimestampTooltip } from './unixTimestamp.ts'

// 10 位秒 / 13 位毫秒都能识别
assert.ok(unixTimestampTooltip('unix_timestamp', 1793345031), '10 位秒级应命中')
assert.ok(unixTimestampTooltip('created_at', 1793345031672), '13 位毫秒应命中')
assert.ok(unixTimestampTooltip('createTime', '1793345031'), '数字字符串应命中')

// 字段名不像时间字段 → 不识别，避免把 10 位 ID 当日期
assert.equal(unixTimestampTooltip('id', 1793345031), null, '普通 id 不识别')
assert.equal(unixTimestampTooltip('description', 1793345031), null, '普通字段不识别')

// 长度 / 年份范围过滤
assert.equal(unixTimestampTooltip('timestamp', 123456789), null, '9 位不识别')
assert.equal(unixTimestampTooltip('timestamp', 9999999999), null, '超出 2100 年不识别')
assert.equal(unixTimestampTooltip('timestamp', 9999999999999), null, '13 位超范围不识别')
assert.equal(unixTimestampTooltip(null, 1793345031), null, '无字段名（数组元素）不识别')

// 文案带本地与 UTC 两种可读形式
const title = unixTimestampTooltip('timestamp', 1793345031)!
assert.match(title, /本地时间/)
assert.match(title, /UTC/)
assert.match(title, /秒级时间戳/)
