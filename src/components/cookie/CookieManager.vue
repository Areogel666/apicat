<template>
  <n-modal v-model:show="show" preset="card" title="Cookie 管理" style="width: 700px; max-width: 95vw">
    <n-tabs type="line" size="small">
      <!-- 全局 Cookie -->
      <n-tab-pane name="global" tab="全局 Cookie">
        <CookieTable
          :cookies="cookieStore.globalCookies"
          @delete="cookieStore.deleteCookie"
          @toggle="(id, enabled) => toggleCookie(id, enabled, 'global')"
        />
        <AddCookieForm scope-type="global" :project-id="null" @created="cookieStore.loadGlobalCookies" />
      </n-tab-pane>

      <!-- 项目 Cookie -->
      <n-tab-pane name="project" tab="项目 Cookie" @click="loadProjectIfNeeded">
        <CookieTable
          :cookies="cookieStore.projectCookies"
          @delete="cookieStore.deleteCookie"
          @toggle="(id, enabled) => toggleCookie(id, enabled, 'project')"
        />
        <AddCookieForm
          scope-type="project"
          :project-id="projectStore.currentProjectId"
          @created="() => projectStore.currentProjectId && cookieStore.loadProjectCookies(projectStore.currentProjectId)"
        />
      </n-tab-pane>
    </n-tabs>

    <template #footer>
      <n-button @click="show = false">关闭</n-button>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { computed, watch } from 'vue'
import { NModal, NButton, NTabs, NTabPane } from 'naive-ui'
import { useCookieStore } from '../../stores/cookie'
import { useProjectStore } from '../../stores/project'
import CookieTable from './CookieTable.vue'
import AddCookieForm from './AddCookieForm.vue'

const props = defineProps<{ show: boolean }>()
const emit = defineEmits<{ 'update:show': [value: boolean] }>()

const show = computed({
  get: () => props.show,
  set: (v) => emit('update:show', v),
})

const cookieStore = useCookieStore()
const projectStore = useProjectStore()

// 打开弹窗时加载全局 Cookie
watch(() => props.show, async (v) => {
  if (v) {
    await cookieStore.loadGlobalCookies()
  }
})

function loadProjectIfNeeded() {
  if (projectStore.currentProjectId) {
    cookieStore.loadProjectCookies(projectStore.currentProjectId)
  }
}

async function toggleCookie(id: number, enabled: boolean, _scope: string) {
  const list = [...cookieStore.globalCookies, ...cookieStore.projectCookies]
  const c = list.find(x => x.id === id)
  if (c) await cookieStore.updateCookie(id, c.value, c.path, enabled ? 1 : 0)
}
</script>
