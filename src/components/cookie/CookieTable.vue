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
        <span style="width:44px; text-align:center">启用</span>
        <span style="width:32px"></span>
      </div>
      <div
        v-for="c in cookies"
        :key="c.id"
        class="table-row"
        :class="{ expired: isExpired(c) }"
      >
        <span class="cell scope" style="width:52px">{{ scopeLabel(c) }}</span>
        <span class="cell" style="flex:1.2" :title="c.domain">{{ c.domain }}</span>
        <span class="cell" style="flex:1">{{ c.name }}</span>
        <span class="cell" style="flex:1.6" :title="c.value">{{ c.value }}</span>
        <span class="cell" style="width:56px">{{ c.path }}</span>
        <span class="cell" style="width:104px" :title="expiryTitle(c)">{{ expiryText(c) }}</span>
        <div style="width:44px; display:flex; justify-content:center">
          <n-checkbox
            :checked="c.enabled === 1"
            @update:checked="(v) => emit('toggle', c.id, v)"
          />
        </div>
        <n-button size="tiny" quaternary style="width:32px; color:var(--color-error)" @click="emit('delete', c.id)">✕</n-button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { NEmpty, NCheckbox, NButton } from 'naive-ui'
import type { CookieItem } from '../../types'

defineProps<{ cookies: CookieItem[] }>()
const emit = defineEmits<{
  delete: [id: number]
  toggle: [id: number, enabled: boolean]
}>()

function scopeLabel(c: CookieItem) {
  if (c.scope_type === 'global') return '全局'
  if (c.scope_type === 'project') return '项目'
  return '环境'
}

function isExpired(c: CookieItem) {
  return !!c.expires_at && c.expires_at <= nowStr()
}

function expiryText(c: CookieItem) {
  if (!c.expires_at) return '会话'
  return isExpired(c) ? '已过期' : c.expires_at.slice(0, 10)
}

function expiryTitle(c: CookieItem) {
  if (!c.expires_at) return '会话 cookie：永不过期，需手动清理'
  return c.expires_at
}

function nowStr() {
  const n = new Date()
  const p = (x: number) => String(x).padStart(2, '0')
  return `${n.getUTCFullYear()}-${p(n.getUTCMonth() + 1)}-${p(n.getUTCDate())} ${p(n.getUTCHours())}:${p(n.getUTCMinutes())}:${p(n.getUTCSeconds())}`
}
</script>

<style scoped>
.cookie-table { margin-bottom: 8px; }
.table-header {
  display: flex;
  padding: 4px 8px;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border-bottom: 1px solid var(--border-base);
}
.table-row {
  display: flex;
  align-items: center;
  padding: 5px 8px;
  font-size: 12px;
  border-bottom: 1px solid var(--border-base);
}
.table-row:hover { background: var(--bg-hover); }
/* Q19-C：过期标灰 */
.table-row.expired { opacity: 0.45; }
.table-row.expired .scope::after {
  content: '·过期';
  color: var(--color-error);
  font-size: 10px;
}
.cell { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; padding-right: 4px; }
.scope { color: var(--text-tertiary); font-size: 11px; }
</style>
