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
        <n-checkbox v-model:checked="form.httpOnly" size="small">HttpOnly</n-checkbox>
        <n-checkbox v-model:checked="form.secure" size="small">Secure</n-checkbox>
        <n-button size="small" type="primary" :disabled="!canSubmit" @click="handleAdd">确定</n-button>
        <n-button size="small" @click="adding = false">取消</n-button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { NButton, NInput, NSelect, NCheckbox } from 'naive-ui'
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
  httpOnly: false,
  secure: false,
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
  // env 层必须同时有项目和环境：只验 envId 的话，「项目已切空 + 环境列表残留」
  // 会写出 project_id=NULL 的行（评审发现的窄边界）
  if (form.scopeType === 'project' && !props.projectId) return false
  if (form.scopeType === 'env' && (!form.envId || !props.projectId)) return false
  return true
})

/**
 * 手填框即时规范化（Q7-C）：贴 `https://x.com/path` 也能对上 URL host。
 *
 * **算法唯一真源是 Rust 的 `cookies::normalize_domain`**，这里必须逐行对齐：
 *   scheme → path/query/fragment → userinfo → port（IPv6 别按冒号切）
 * 顺序敏感：先剥 userinfo 再切 path 的话，`?mail=x@y.com` 会被截成 `y.com`。
 * 后端 create_cookie 会再规范化一次兜底，此处只为即时反馈。
 */
function normalizeDomain() {
  let s = form.domain.trim()
  const scheme = s.indexOf('://')
  if (scheme >= 0) s = s.slice(scheme + 3)
  // 1. 先切 path / query / fragment
  for (const sep of ['/', '?', '#']) {
    const i = s.indexOf(sep)
    if (i >= 0) { s = s.slice(0, i); break }
  }
  // 2. 再剥 userinfo
  const at = s.lastIndexOf('@')
  if (at >= 0) s = s.slice(at + 1)
  // 3. 剥端口。IPv6 含多个冒号不能按第一个冒号切
  if (s.startsWith('[')) {
    const end = s.indexOf(']')
    if (end >= 0) s = s.slice(0, end + 1)
  } else if ((s.match(/:/g) ?? []).length === 1) {
    s = s.slice(0, s.indexOf(':'))
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
    httpOnly: form.httpOnly,
    secure: form.secure,
  })
  Object.assign(form, {
    domain: '', name: '', value: '', path: '/', envId: null,
    httpOnly: false, secure: false,
  })
  adding.value = false
  emit('created')
}
</script>

<style scoped>
/* 全部走项目主题 token（见 CLAUDE.md「主题系统」），硬编码 px 不随风格档位变 */
.add-cookie-form { margin-top: var(--spacing-sm); }
.form-row {
  display: flex;
  gap: var(--spacing-xs);
  align-items: center;
  margin-bottom: var(--spacing-xs);
}
</style>
