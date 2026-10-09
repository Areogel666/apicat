<template>
  <div v-if="expanded" ref="rootEl" class="cookie-editor">
    <!-- 分区标题：明确区分「这是编辑区」以及「在编辑哪一条」 -->
    <div class="editor-title">{{ editorTitle }}</div>
    <!-- 第一行：作用域（编辑时锁定——后端不支持改作用域，要改只能删了重建） -->
    <div class="form-row">
      <n-select
        v-model:value="form.scopeType"
        size="small"
        :options="scopeOptions"
        :disabled="isEditing"
        style="width:110px; flex-shrink:0"
      />
      <!-- 选环境即定下 host（Q7-C：环境下拉为主） -->
      <n-select
        v-if="form.scopeType === 'env'"
        v-model:value="form.envId"
        size="small"
        :options="envOptions"
        :disabled="isEditing"
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
      <n-date-picker
        v-model:value="form.expiresAt"
        type="datetime"
        size="small"
        clearable
        placeholder="过期时间（空=会话）"
        style="width:180px; flex-shrink:0"
      />
      <n-checkbox v-model:checked="form.httpOnly" size="small" title="仅记录：ApiCat 不是浏览器，没有页面脚本会去读 cookie，故不影响发送">HttpOnly</n-checkbox>
      <n-checkbox v-model:checked="form.secure" size="small" title="仅在 https / wss 请求中发送，明文 http 请求会跳过">Secure</n-checkbox>
      <n-button size="small" type="primary" :disabled="!canSubmit" :loading="saving" @click="handleSubmit">
        {{ isEditing ? '保存' : '确定' }}
      </n-button>
      <n-button size="small" @click="handleCancel">取消</n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, nextTick, watch } from 'vue'
import { NButton, NInput, NSelect, NCheckbox, NDatePicker, useMessage } from 'naive-ui'
import { useCookieStore, type CookieScope } from '../../stores/cookie'
import { useEnvironmentStore } from '../../stores/environment'
import { tsToUtcString, utcStringToTs } from '../../utils/time'
import type { CookieItem } from '../../types'

const props = defineProps<{
  /** 默认作用域（弹窗按筛选项预填） */
  defaultScope?: CookieScope
  projectId: number | null
  /** 新建模式（由父组件的「+ 添加 Cookie」开启） */
  creating?: boolean
  /** 编辑中的 cookie；非空即编辑模式（作用域锁定，提交走 update） */
  editing?: CookieItem | null
}>()
const emit = defineEmits<{ saved: []; cancel: [] }>()

const cookieStore = useCookieStore()
const envStore = useEnvironmentStore()
const message = useMessage()
const saving = ref(false)
const rootEl = ref<HTMLElement | null>(null)

const isEditing = computed(() => !!props.editing)
const expanded = computed(() => !!props.creating || isEditing.value)

const editorTitle = computed(() => {
  const c = props.editing
  if (!c) return '新建 Cookie'
  return `编辑 · ${scopeLabel(c.scope_type)} · ${c.domain} · ${c.name}`
})

const form = reactive({
  scopeType: (props.defaultScope ?? 'global') as CookieScope,
  envId: null as number | null,
  domain: '',
  name: '',
  value: '',
  path: '/',
  /** 毫秒时间戳（n-date-picker 的值）；null = 会话 cookie */
  expiresAt: null as number | null,
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
  // 编辑态作用域锁定，原值必然自洽，无需再验
  if (isEditing.value) return true
  // env 层必须同时有项目和环境：只验 envId 的话，「项目已切空 + 环境列表残留」
  // 会写出 project_id=NULL 的行（评审发现的窄边界）
  if (form.scopeType === 'project' && !props.projectId) return false
  if (form.scopeType === 'env' && (!form.envId || !props.projectId)) return false
  return true
})

function scopeLabel(s: string) {
  return s === 'global' ? '全局' : s === 'project' ? '项目' : '环境'
}

/** 清空内容字段，不动 scopeType（连续新建时保留上次选的作用域） */
function resetForm() {
  Object.assign(form, {
    envId: null, domain: '', name: '', value: '', path: '/',
    expiresAt: null, httpOnly: false, secure: false,
  })
}

function useDefaultScope() {
  form.scopeType = props.defaultScope ?? 'global'
}

/**
 * 展开时把自己滚进视野。
 * 列表长的时候编辑区落在视口外，点 ✎ 会像「没反应」——这是本次分区改造要解决的痛点之一。
 */
function scrollIntoView() {
  nextTick(() => rootEl.value?.scrollIntoView({ behavior: 'smooth', block: 'nearest' }))
}

/** 编辑对象变化 → 预填；变空 → 复位并回到弹窗筛选的默认作用域 */
watch(() => props.editing, (c) => {
  if (!c) {
    resetForm()
    useDefaultScope()
    return
  }
  Object.assign(form, {
    scopeType: c.scope_type as CookieScope,
    envId: c.env_id,
    domain: c.domain,
    name: c.name,
    value: c.value,
    path: c.path,
    expiresAt: utcStringToTs(c.expires_at),
    httpOnly: c.http_only === 1,
    secure: c.secure === 1,
  })
  scrollIntoView()
}, { immediate: true })

/** 新建模式开启 → 复位并预填默认作用域 */
watch(() => props.creating, (creating) => {
  if (!creating) return
  resetForm()
  useDefaultScope()
  scrollIntoView()
})

function handleCancel() {
  resetForm()
  emit('cancel')
}

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

async function handleSubmit() {
  if (!canSubmit.value || saving.value) return
  normalizeDomain()
  saving.value = true
  try {
    // 过期时间：编辑时传空串 = 清回会话 cookie（后端三态约定）；新建时空 = null
    const expiresAt = form.expiresAt == null
      ? (isEditing.value ? '' : null)
      : tsToUtcString(form.expiresAt)

    if (isEditing.value && props.editing) {
      await cookieStore.updateCookie(props.editing.id, {
        domain: form.domain,
        name: form.name,
        value: form.value,
        path: form.path || '/',
        expiresAt,
        httpOnly: form.httpOnly,
        secure: form.secure,
      })
    } else {
      await cookieStore.createCookie({
        scopeType: form.scopeType,
        projectId: form.scopeType === 'global' ? null : props.projectId,
        envId: form.scopeType === 'env' ? form.envId : null,
        domain: form.domain,
        name: form.name,
        value: form.value,
        path: form.path || '/',
        expiresAt,
        httpOnly: form.httpOnly,
        secure: form.secure,
      })
    }
    resetForm()
    emit('saved')
  } catch (e) {
    // 重复键由后端拦截返回友好文案（全局/项目层的表唯一约束在 SQLite 下失效，
    // 查重只能由 create_cookie_impl 在应用层做）
    message.error(String(e))
  } finally {
    saving.value = false
  }
}
</script>

<style scoped>
/* 全部走项目主题 token（见 CLAUDE.md「主题系统」），硬编码 px 不随风格档位变 */
/* 编辑区：与上方列表用分隔线 + 标题明确切开 */
.cookie-editor {
  margin-top: var(--spacing-md);
  padding-top: var(--spacing-md);
  border-top: 1px solid var(--border-base);
}
.editor-title {
  font-size: var(--font-size-sm);
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: var(--spacing-sm);
}
.form-row {
  display: flex;
  gap: var(--spacing-xs);
  align-items: center;
  margin-bottom: var(--spacing-xs);
}
</style>
