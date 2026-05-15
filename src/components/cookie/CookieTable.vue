<template>
  <div class="cookie-table">
    <n-empty v-if="!cookies.length" description="暂无 Cookie" size="small" style="margin: 16px 0" />
    <div v-else>
      <div class="table-header">
        <span style="flex:1.2">域名</span>
        <span style="flex:1">名称</span>
        <span style="flex:2">值</span>
        <span style="width:50px; text-align:center">启用</span>
        <span style="width:32px"></span>
      </div>
      <div v-for="c in cookies" :key="c.id" class="table-row">
        <span class="cell" style="flex:1.2">{{ c.domain }}</span>
        <span class="cell" style="flex:1">{{ c.name }}</span>
        <span class="cell" style="flex:2">{{ c.value }}</span>
        <div style="width:50px; display:flex; justify-content:center">
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
.cell { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; padding-right: 4px; }
</style>
