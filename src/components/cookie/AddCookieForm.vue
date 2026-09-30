<template>
  <div class="add-cookie-form">
    <div v-if="!adding" style="margin-top: 4px">
      <n-button size="small" dashed style="width:100%" @click="adding = true">
        + 添加 Cookie
      </n-button>
    </div>
    <div v-else>
      <!-- 第一行：作用域（Q14-B 改一张表后，作用域在新建时选定） -->
      <div class="form-row">
        <n-select
          v-model:value="form.scopeType"
          size="small"
          :options="scopeOptions"
          style="width:110px; flex-shrink:0"
        />
        <!-- 选环境即定下 host（Q7-C：环境下拉为主） -->
        <n-select
          v-if="form.scopeType === 'env'"
          v-model:value="form.envId"
          size="small"
          :options="envOptions"
          placeholder="选环境"
          style="width:150px; flex-shrink:0"
          @update:value="onEnvChange"
        />
        <n-input
          v-model:value="form.domain"
          size="small"
          placeholder="域名 如 api.example.com（可贴完整 URL）"
          style="flex:1.2"
          @blur="normalizeDomain"
        />
        <n-input v-model:value="form.name" size="small" placeholder="名称" style="flex:1" />
      </div>
      <!-- 第二行 -->
      <div class="form-row">
        <n-input v-model:value="form.value" size="small" placeholder="值" style="flex:2" />
        <n-input v-model:value="form.path" size="small" placeholder="路径 /" style="width:70px; flex-shrink:0" />
        <n-button size="small" type="primary" :disabled="!canSubmit" @click="handleAdd">确定</n-button>
        <n-button size="small" @click="adding = false">取消</n-button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { NButton, NInput, NSelect } from 'naive-ui'
import { useCookieStore, type CookieScope } from '../../stores/cookie'
import { useEnvironmentStore } from '../../stores/environment'

const props = defineProps<{
  /** 默认作用域（弹窗按筛选项预填） */
  defaultScope?: CookieScope
  projectId: number | null
}>()
const emit = defineEmits<{ created: [] }>()

const cookieStore = useCookieStore()
const envStore = useEnvironmentStore()
const adding = ref(false)
const form = reactive({
  scopeType: (props.defaultScope ?? 'global') as CookieScope,
  envId: null as number | null,
  domain: '',
  name: '',
  value: '',
  path: '/',
})

const scopeOptions = [
  { label: '全局', value: 'global' },
  { label: '项目', value: 'project' },
  { label: '环境', value: 'env' },
]

const envOptions = computed(() =>
  envStore.environments.map(e => ({ label: e.name, value: e.id })),
)

const canSubmit = computed(() => {
  if (!form.domain || !form.name) return false
  if (form.scopeType === 'project' && !props.projectId) return false
  if (form.scopeType === 'env' && !form.envId) return false
  return true
})

/**
 * 手填框自动规范化（Q7-C）：贴 `https://x.com/path` 也能对上 URL host。
 * 后端 create_cookie 也会 normalize，这里是即时反馈，避免用户看着「填了没生效」。
 */
function normalizeDomain() {
  let s = form.domain.trim()
  const scheme = s.indexOf('://')
  if (scheme >= 0) s = s.slice(scheme + 3)
  const at = s.lastIndexOf('@')
  if (at >= 0) s = s.slice(at + 1)
  for (const sep of ['/', '?', '#']) {
    const i = s.indexOf(sep)
    if (i >= 0) s = s.slice(0, i)
  }
  if (!s.startsWith('[')) {
    const colon = s.indexOf(':')
    if (colon >= 0) s = s.slice(0, colon)
  }
  form.domain = s.replace(/^\.+/, '').toLowerCase()
}

/** 选环境 → 从 base_url 推 host 预填 domain（Q7-C） */
function onEnvChange(envId: number | null) {
  const env = envStore.environments.find(e => e.id === envId)
  if (!env?.base_url) return
  form.domain = env.base_url
  normalizeDomain()
}

onMounted(async () => {
  // 环境列表是按项目加载的；弹窗打开时确保已就绪
  if (props.projectId && envStore.environments.length === 0) {
    try {
      await envStore.loadEnvironments(props.projectId)
    } catch {
      /* 环境加载失败不阻断添加 */
    }
  }
})

async function handleAdd() {
  if (!canSubmit.value) return
  normalizeDomain()
  await cookieStore.createCookie({
    scopeType: form.scopeType,
    projectId: form.scopeType === 'global' ? null : props.projectId,
    envId: form.scopeType === 'env' ? form.envId : null,
    domain: form.domain,
    name: form.name,
    value: form.value,
    path: form.path || '/',
  })
  Object.assign(form, { domain: '', name: '', value: '', path: '/', envId: null })
  adding.value = false
  emit('created')
}
</script>

<style scoped>
.add-cookie-form { margin-top: 8px; }
.form-row { display: flex; gap: 4px; align-items: center; margin-bottom: 4px; }
</style>
