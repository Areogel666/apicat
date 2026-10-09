<template>
  <div class="cookie-table">
    <n-empty v-if="!cookies.length" description="暂无 Cookie" size="small" style="margin: 16px 0" />
    <div v-else>
      <div class="table-header">
        <span style="width:52px">作用域</span>
        <span style="flex:1.2">域名</span>
        <span style="flex:1">名称</span>
        <span style="flex:1.6">值</span>
        <span style="width:56px">路径</span>
        <span style="width:104px">过期</span>
        <span style="width:36px; text-align:center" title="HttpOnly（仅记录，不影响发送）">Http</span>
        <span style="width:36px; text-align:center" title="Secure（仅在 https / wss 请求中发送）">Sec</span>
        <span style="width:44px; text-align:center">启用</span>
        <span style="width:64px"></span>
      </div>
      <div
        v-for="c in cookies"
        :key="c.id"
        class="table-row"
        :class="{ expired: isExpired(c), editing: c.id === editingId }"
      >
        <span class="cell scope" style="width:52px">{{ scopeLabel(c) }}</span>
        <span class="cell" style="flex:1.2" :title="c.domain">{{ c.domain }}</span>
        <span class="cell" style="flex:1">{{ c.name }}</span>
        <span class="cell" style="flex:1.6" :title="c.value">{{ c.value }}</span>
        <span class="cell" style="width:56px">{{ c.path }}</span>
        <span class="cell" style="width:104px" :title="expiryTitle(c)">{{ expiryText(c) }}</span>
        <span class="cell flag" style="width:36px; text-align:center">{{ c.http_only ? '✓' : '' }}</span>
        <span class="cell flag" style="width:36px; text-align:center">{{ c.secure ? '✓' : '' }}</span>
        <div style="width:44px; display:flex; justify-content:center">
          <n-checkbox
            :checked="c.enabled === 1"
            @update:checked="(v) => emit('toggle', c.id, v)"
          />
        </div>
        <div style="width:64px; display:flex; justify-content:center; gap:2px">
          <n-button size="tiny" quaternary style="width:28px" title="编辑" @click="emit('edit', c)">✎</n-button>
          <n-button size="tiny" quaternary style="width:28px; color:var(--color-error)" title="删除" @click="emit('delete', c.id)">✕</n-button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { NEmpty, NCheckbox, NButton } from 'naive-ui'
import type { CookieItem } from '../../types'
import { nowUtcString } from '../../utils/time'

defineProps<{
  cookies: CookieItem[]
  /** 正在编辑的行 id —— 高亮该行，与下方编辑区呼应 */
  editingId?: number | null
}>()
const emit = defineEmits<{
  delete: [id: number]
  edit: [cookie: CookieItem]
  toggle: [id: number, enabled: boolean]
}>()

function scopeLabel(c: CookieItem) {
  if (c.scope_type === 'global') return '全局'
  if (c.scope_type === 'project') return '项目'
  return '环境'
}

function isExpired(c: CookieItem) {
  return !!c.expires_at && c.expires_at <= nowUtcString()
}

function expiryText(c: CookieItem) {
  if (!c.expires_at) return '会话'
  return isExpired(c) ? '已过期' : c.expires_at.slice(0, 10)
}

function expiryTitle(c: CookieItem) {
  if (!c.expires_at) return '会话 cookie：永不过期，需手动清理'
  return c.expires_at
}
</script>

<style scoped>
/* 全部走项目主题 token（见 CLAUDE.md「主题系统」），硬编码 px 不随风格档位变 */
.cookie-table { margin-bottom: var(--spacing-sm); }
.table-header {
  display: flex;
  align-items: center;
  min-height: var(--row-height);
  padding: 0 var(--spacing-sm);
  font-size: var(--font-size-sm);
  font-weight: 600;
  color: var(--text-tertiary);
  border-bottom: 1px solid var(--border-base);
}
.table-row {
  display: flex;
  align-items: center;
  min-height: var(--row-height);
  padding: var(--spacing-xs) var(--spacing-sm);
  font-size: var(--font-size-base);
  border-bottom: 1px solid var(--border-base);
}
.table-row:hover { background: var(--bg-hover); }
/* 编辑中的行：必须放在 :hover 之后——同特异性下后定义者胜，
   否则鼠标一悬停高亮就被 hover 底色盖掉 */
.table-row.editing {
  background: var(--bg-selected);
  box-shadow: inset 2px 0 0 var(--color-primary);
}
/* Q19-C：过期标灰 */
.table-row.expired { opacity: 0.45; }
.table-row.expired .scope::after {
  content: '·过期';
  color: var(--color-error);
  font-size: var(--font-size-sm);
}
.cell {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  padding-right: var(--spacing-xs);
}
.scope, .flag { color: var(--text-tertiary); font-size: var(--font-size-sm); }
</style>
