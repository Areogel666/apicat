<template>
  <main class="main-panel">
    <!-- 上半：请求编辑区 -->
    <div class="request-area">
      <!-- URL 栏 -->
      <div class="url-bar">
        <n-select
          v-model:value="method"
          :options="methodOptions"
          size="medium"
          style="width: 110px; flex-shrink: 0"
        />
        <!-- URL 输入框 + {{var}} 高亮层 -->
        <div class="url-input-wrap">
          <!-- 高亮覆盖层：pointer-events:none，仅视觉，{{var}} 用橙色底色标注 -->
          <div class="url-highlight-layer" aria-hidden="true" v-html="urlHighlighted" />
          <n-input
            v-model:value="url"
            placeholder="输入请求 URL，如 https://api.example.com/users"
            size="medium"
            class="url-input-transparent"
            @keyup.enter="handleSend"
          />
        </div>
        <n-button
          type="primary"
          size="medium"
          style="flex-shrink: 0; min-width: 72px"
          :loading="responseStore.loading"
          @click="handleSend"
        >
          发 送
        </n-button>
        <n-button
          size="medium"
          style="flex-shrink: 0"
          :disabled="!url || responseStore.loading"
          title="压测"
          @click="showStressConfig = true"
        >
          ⚡ 压测
        </n-button>
      </div>

      <!-- 请求配置 Tabs -->
      <n-tabs type="line" size="small" class="request-tabs">
        <n-tab-pane name="params" tab="Params">
          <div class="params-editor">
            <!-- Path Params（由 URL 自动提取）-->
            <template v-if="parsedUrl && parsedUrl.pathParams.length > 0">
              <div class="params-section-label">Path Params</div>
              <div v-for="p in parsedUrl.pathParams" :key="p.key" class="param-row">
                <n-tag size="small" type="info" style="width:120px; text-align:center; flex-shrink:0">{{ p.key }}</n-tag>
                <n-input v-model:value="pathParamValues[p.key]" size="small" style="flex:1" placeholder="值" />
              </div>
              <n-divider style="margin: 8px 0" />
            </template>

            <!-- Query Params（三模式切换）-->
            <div class="params-mode-bar">
              <span class="params-section-label" style="margin-bottom:0">Query Params</span>
              <div class="mode-tabs">
                <span :class="['mode-tab', queryMode==='table' && 'active']" @click="switchQueryMode('table')">表格</span>
                <span :class="['mode-tab', queryMode==='kv' && 'active']" @click="switchQueryMode('kv')">KV 文本</span>
                <span :class="['mode-tab', queryMode==='json' && 'active']" @click="switchQueryMode('json')">JSON</span>
              </div>
            </div>

            <!-- 表格模式 -->
            <template v-if="queryMode === 'table'">
              <n-empty v-if="!queryParams.length" description="暂无参数" size="small" />
              <div v-for="(q, idx) in queryParams" :key="idx" class="param-row">
                <n-checkbox v-model:checked="q.enabled" />
                <n-input v-model:value="q.key" size="small" style="width:140px; flex-shrink:0" placeholder="Key" />
                <n-input v-model:value="q.value" size="small" style="flex:1" placeholder="Value" />
                <n-button size="tiny" quaternary @click="queryParams.splice(idx, 1)">✕</n-button>
              </div>
              <n-button size="small" dashed style="margin-top:4px; width:100%" @click="addQueryParam">
                + 添加 Query Param
              </n-button>
            </template>

            <!-- KV 文本模式 -->
            <n-input
              v-else-if="queryMode === 'kv'"
              v-model:value="queryKvText"
              type="textarea"
              :rows="6"
              placeholder="Key: Value（每行一条，# 开头为注释）"
              style="font-family: monospace; font-size: 12px; margin-top: 4px"
            />

            <!-- JSON 模式 -->
            <n-input
              v-else-if="queryMode === 'json'"
              v-model:value="queryJsonText"
              type="textarea"
              :rows="6"
              placeholder='{"key": "value"}'
              style="font-family: monospace; font-size: 12px; margin-top: 4px"
            />
          </div>
        </n-tab-pane>

        <n-tab-pane name="headers" tab="Headers">
          <div class="params-editor">
            <div class="params-mode-bar">
              <span class="params-section-label" style="margin-bottom:0">Headers</span>
              <div style="display:flex;gap:6px;align-items:center">
                <n-button
                  v-if="headerTemplateStore.getEnabledItems().length > 0"
                  size="tiny"
                  secondary
                  title="将公共 Headers 模板批量插入到当前编辑区（跳过已存在的 Key）"
                  @click="applyHeaderTemplate"
                >
                  📋 应用模板
                </n-button>
                <div class="mode-tabs">
                  <span :class="['mode-tab', headerMode==='table' && 'active']" @click="switchHeaderMode('table')">表格</span>
                  <span :class="['mode-tab', headerMode==='kv' && 'active']" @click="switchHeaderMode('kv')">KV 文本</span>
                  <span :class="['mode-tab', headerMode==='json' && 'active']" @click="switchHeaderMode('json')">JSON</span>
                </div>
              </div>
            </div>

            <template v-if="headerMode === 'table'">
              <n-empty v-if="!requestHeaders.length" description="暂无 Headers" size="small" />
              <div v-for="(h, idx) in requestHeaders" :key="idx" class="param-row">
                <n-checkbox v-model:checked="h.enabled" />
                <n-input v-model:value="h.key" size="small" style="width:160px; flex-shrink:0" placeholder="Header 名" />
                <n-input v-model:value="h.value" size="small" style="flex:1" placeholder="值" />
                <n-button size="tiny" quaternary @click="requestHeaders.splice(idx, 1)">✕</n-button>
              </div>
              <n-button size="small" dashed style="margin-top:4px; width:100%" @click="addHeader">
                + 添加 Header
              </n-button>
            </template>

            <n-input
              v-else-if="headerMode === 'kv'"
              v-model:value="headerKvText"
              type="textarea"
              :rows="6"
              placeholder="Header-Name: value（每行一条）"
              style="font-family: monospace; font-size: 12px; margin-top: 4px"
            />

            <n-input
              v-else-if="headerMode === 'json'"
              v-model:value="headerJsonText"
              type="textarea"
              :rows="6"
              placeholder='{"Content-Type": "application/json"}'
              style="font-family: monospace; font-size: 12px; margin-top: 4px"
            />
          </div>
        </n-tab-pane>

        <n-tab-pane name="body" tab="Body">
          <div class="params-editor">
            <div class="params-section-label" style="margin-bottom:8px">Body 类型</div>
            <n-radio-group v-model:value="bodyType" size="small" style="margin-bottom: 8px">
              <n-radio-button value="none">None</n-radio-button>
              <n-radio-button value="raw_json">JSON</n-radio-button>
              <n-radio-button value="raw_text">Text</n-radio-button>
              <n-radio-button value="form_urlencoded">Form URL</n-radio-button>
              <n-radio-button value="form_data">Form Data</n-radio-button>
            </n-radio-group>

            <!-- 文本类 body -->
            <n-input
              v-if="bodyType === 'raw_json' || bodyType === 'raw_text' || bodyType === 'form_urlencoded'"
              v-model:value="bodyContent"
              type="textarea"
              :rows="8"
              placeholder="请求体内容"
              style="font-family: monospace; font-size: 12px"
            />

            <!-- form-data KV 表格 -->
            <template v-if="bodyType === 'form_data'">
              <n-empty v-if="!formDataParams.length" description="暂无字段" size="small" />
              <div v-for="(f, idx) in formDataParams" :key="idx" class="param-row">
                <n-checkbox v-model:checked="f.enabled" />
                <n-input v-model:value="f.key" size="small" style="width:140px; flex-shrink:0" placeholder="字段名" />
                <n-input v-model:value="f.value" size="small" style="flex:1" placeholder="值" />
                <n-button size="tiny" quaternary @click="formDataParams.splice(idx, 1)">✕</n-button>
              </div>
              <n-button size="small" dashed style="margin-top:4px; width:100%" @click="addFormDataField">
                + 添加字段
              </n-button>
            </template>
          </div>
        </n-tab-pane>

        <n-tab-pane name="auth" tab="Auth">
          <div class="params-editor">
            <!-- Auth 类型选择 -->
            <div class="params-mode-bar" style="margin-bottom:10px">
              <span class="params-section-label">认证类型</span>
              <n-select
                v-model:value="authType"
                :options="authTypeOptions"
                size="small"
                style="width: 180px"
                @update:value="onAuthTypeChange"
              />
            </div>

            <!-- None -->
            <div v-if="authType === 'none'" style="color: var(--n-text-color-3,#999); font-size:13px; padding:8px 0">
              不使用认证
            </div>

            <!-- Bearer Token -->
            <template v-if="authType === 'bearer'">
              <div class="param-row">
                <span style="width:80px;font-size:13px;flex-shrink:0">Token</span>
                <n-input
                  v-model:value="authBearer"
                  placeholder="Bearer token 值"
                  size="small"
                  style="flex:1; font-family:monospace"
                  @update:value="syncAuthConfig"
                />
              </div>
              <div style="font-size:11px;color:var(--n-text-color-3,#999);padding:4px 0">
                将自动添加 <code>Authorization: Bearer &lt;token&gt;</code> 请求头
              </div>
            </template>

            <!-- Basic Auth -->
            <template v-if="authType === 'basic'">
              <div class="param-row">
                <span style="width:80px;font-size:13px;flex-shrink:0">用户名</span>
                <n-input
                  v-model:value="authBasicUser"
                  placeholder="Username"
                  size="small"
                  style="flex:1"
                  @update:value="syncAuthConfig"
                />
              </div>
              <div class="param-row" style="margin-top:6px">
                <span style="width:80px;font-size:13px;flex-shrink:0">密码</span>
                <n-input
                  v-model:value="authBasicPass"
                  placeholder="Password"
                  size="small"
                  type="password"
                  show-password-on="click"
                  style="flex:1"
                  @update:value="syncAuthConfig"
                />
              </div>
              <div style="font-size:11px;color:var(--n-text-color-3,#999);padding:4px 0">
                将自动添加 <code>Authorization: Basic &lt;base64&gt;</code> 请求头
              </div>
            </template>

            <!-- API Key -->
            <template v-if="authType === 'api_key'">
              <div class="param-row">
                <span style="width:80px;font-size:13px;flex-shrink:0">Key 名称</span>
                <n-input
                  v-model:value="authApiKeyName"
                  placeholder="X-Api-Key"
                  size="small"
                  style="flex:1"
                  @update:value="syncAuthConfig"
                />
              </div>
              <div class="param-row" style="margin-top:6px">
                <span style="width:80px;font-size:13px;flex-shrink:0">Key 值</span>
                <n-input
                  v-model:value="authApiKeyValue"
                  placeholder="api key 值"
                  size="small"
                  style="flex:1; font-family:monospace"
                  @update:value="syncAuthConfig"
                />
              </div>
              <div class="param-row" style="margin-top:6px">
                <span style="width:80px;font-size:13px;flex-shrink:0">添加到</span>
                <n-radio-group v-model:value="authApiKeyIn" size="small" @update:value="syncAuthConfig">
                  <n-radio value="header">Header</n-radio>
                  <n-radio value="query">Query Param</n-radio>
                </n-radio-group>
              </div>
            </template>
          </div>
        </n-tab-pane>
      </n-tabs>
    </div>

    <!-- 分隔线 -->
    <div class="divider" />

    <!-- 用例选择栏（Send 后出现，低干扰）-->
    <TestCaseBar
      :test-cases="testCaseStore.getByRequestId(requestStore.activeRequest?.id ?? 0)"
      :active-id="testCaseStore.activeTestCaseId"
      :params-dirty="paramsDirty"
      @activate="handleActivateTestCase"
      @create="handleCreateTestCase"
      @rename="handleRenameTestCase"
      @toggle-star="handleToggleStar"
      @delete="handleDeleteTestCase"
      @save-to-active="handleSaveToActive"
      @save-as-new="handleSaveAsNew"
      @dismiss-dirty="paramsDirty = false"
    />

    <!-- 下半：响应区 -->
    <ResponsePanel @refill="handleRefill" />

    <!-- 压测配置弹窗 -->
    <StressConfigModal
      v-model:show="showStressConfig"
      @start="handleStartStress"
    />

    <!-- 压测结果弹窗 -->
    <StressResultPanel v-model:show="showStressResult" />
  </main>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import {
  NSelect, NInput, NButton, NTabs, NTabPane, NEmpty,
  NTag, NDivider, NCheckbox, NRadioGroup, NRadioButton, NRadio,
} from 'naive-ui'
import { parseUrl, buildUrl } from '../../utils/urlParser'
import { parseKvText, toKvText, parseJsonToParams, toJsonText } from '../../utils/paramParser'
import { useRequestStore } from '../../stores/request'
import { useResponseStore } from '../../stores/response'
import { useHistoryStore } from '../../stores/history'
import { useEnvironmentStore } from '../../stores/environment'
import { useProjectStore } from '../../stores/project'
import { useTestCaseStore } from '../../stores/testCase'
import { useStressStore } from '../../stores/stress'
import { useHeaderTemplateStore } from '../../stores/headerTemplate'
import ResponsePanel from '../response/ResponsePanel.vue'
import TestCaseBar from '../testcase/TestCaseBar.vue'
import StressConfigModal from '../stress/StressConfigModal.vue'
import StressResultPanel from '../stress/StressResultPanel.vue'
import type { ParamItem, ParsedUrl, StressConfig } from '../../types'

type ParamMode = 'table' | 'kv' | 'json'

// ── Query Params 模式 ──────────────────────────────────────────
const queryMode = ref<ParamMode>('table')
const queryKvText = ref('')
const queryJsonText = ref('')

function switchQueryMode(newMode: ParamMode) {
  // 先把当前模式内容同步到 queryParams（table 是 source of truth）
  if (queryMode.value === 'kv') {
    queryParams.value = parseKvText(queryKvText.value)
  } else if (queryMode.value === 'json') {
    queryParams.value = parseJsonToParams(queryJsonText.value)
  }
  // 再渲染目标模式
  if (newMode === 'kv') {
    queryKvText.value = toKvText(queryParams.value)
  } else if (newMode === 'json') {
    queryJsonText.value = toJsonText(queryParams.value)
  }
  queryMode.value = newMode
}

// ── Headers 模式 ───────────────────────────────────────────────
const headerMode = ref<ParamMode>('table')
const headerKvText = ref('')
const headerJsonText = ref('')

function switchHeaderMode(newMode: ParamMode) {
  if (headerMode.value === 'kv') {
    requestHeaders.value = parseKvText(headerKvText.value)
  } else if (headerMode.value === 'json') {
    requestHeaders.value = parseJsonToParams(headerJsonText.value)
  }
  if (newMode === 'kv') {
    headerKvText.value = toKvText(requestHeaders.value)
  } else if (newMode === 'json') {
    headerJsonText.value = toJsonText(requestHeaders.value)
  }
  headerMode.value = newMode
}

const requestStore = useRequestStore()
const responseStore = useResponseStore()
const historyStore = useHistoryStore()
const envStore = useEnvironmentStore()
const projectStore = useProjectStore()
const testCaseStore = useTestCaseStore()
const headerTemplateStore = useHeaderTemplateStore()

// ── 请求编辑区状态 ────────────────────────────────────────────
const method = ref('GET')
const url = ref('')
const pathParamValues = ref<Record<string, string>>({})
const queryParams = ref<ParamItem[]>([])
const requestHeaders = ref<ParamItem[]>([])
const bodyType = ref('none')
const bodyContent = ref('')

// form-data KV 字段（bodyType === 'form_data' 时使用）
const formDataParams = ref<ParamItem[]>([])

function addFormDataField() {
  formDataParams.value.push({ key: '', value: '', enabled: true })
}

/** 当 bodyType 为 form_data 时，将 formDataParams 序列化到 bodyContent */
function syncFormData() {
  if (bodyType.value === 'form_data') {
    bodyContent.value = JSON.stringify(formDataParams.value)
  }
}

// ── Auth 状态 ─────────────────────────────────────────────────
const authType = ref('none')
const authBearer = ref('')
const authBasicUser = ref('')
const authBasicPass = ref('')
const authApiKeyName = ref('X-Api-Key')
const authApiKeyValue = ref('')
const authApiKeyIn = ref<'header' | 'query'>('header')

const authTypeOptions = [
  { label: '无认证', value: 'none' },
  { label: 'Bearer Token', value: 'bearer' },
  { label: 'Basic Auth', value: 'basic' },
  { label: 'API Key', value: 'api_key' },
]

/** 切换 auth 类型时，从接口数据恢复字段（或重置） */
function onAuthTypeChange(val: string) {
  authType.value = val
  syncAuthConfig()
}

/** 把当前 auth 子字段合并为 auth_config JSON 并保存到接口 */
function syncAuthConfig() {
  let cfg = '{}'
  if (authType.value === 'bearer') {
    cfg = JSON.stringify({ token: authBearer.value })
  } else if (authType.value === 'basic') {
    cfg = JSON.stringify({ username: authBasicUser.value, password: authBasicPass.value })
  } else if (authType.value === 'api_key') {
    cfg = JSON.stringify({ key: authApiKeyName.value, value: authApiKeyValue.value, in: authApiKeyIn.value })
  }
  // 静默保存到当前接口（不影响发请求，发请求时从 ref 读取）
  const req = requestStore.activeRequest
  if (req) {
    requestStore.updateRequest(req.id, { auth_type: authType.value, auth_config: cfg })
  }
}

/** 从 auth_type / auth_config 字符串恢复子字段 */
function loadAuthFields(type: string, configStr: string) {
  authType.value = type || 'none'
  const cfg = (() => { try { return JSON.parse(configStr || '{}') } catch { return {} } })()
  authBearer.value = cfg.token ?? ''
  authBasicUser.value = cfg.username ?? ''
  authBasicPass.value = cfg.password ?? ''
  authApiKeyName.value = cfg.key ?? 'X-Api-Key'
  authApiKeyValue.value = cfg.value ?? ''
  authApiKeyIn.value = cfg.in === 'query' ? 'query' : 'header'
}

const methodOptions = [
  { label: 'GET',     value: 'GET' },
  { label: 'POST',    value: 'POST' },
  { label: 'PUT',     value: 'PUT' },
  { label: 'DELETE',  value: 'DELETE' },
  { label: 'PATCH',   value: 'PATCH' },
  { label: 'HEAD',    value: 'HEAD' },
  { label: 'OPTIONS', value: 'OPTIONS' },
]

// ── 监听激活接口变化，同步到编辑区 ───────────────────────────
watch(() => requestStore.activeRequest, async (req) => {
  if (req) {
    url.value = req.url
    method.value = req.method
    bodyType.value = req.body_type || 'none'
    bodyContent.value = req.body || ''
    // 加载 form-data 字段
    if (req.body_type === 'form_data') {
      try { formDataParams.value = JSON.parse(req.body || '[]') } catch { formDataParams.value = [] }
    } else {
      formDataParams.value = []
    }

    // 解析存储的 params/headers JSON
    try { queryParams.value = JSON.parse(req.params) } catch { queryParams.value = [] }
    try { requestHeaders.value = JSON.parse(req.headers) } catch { requestHeaders.value = [] }

    // 切换接口时重置模式为 table、清空 dirty 状态
    queryMode.value = 'table'
    headerMode.value = 'table'
    paramsDirty.value = false
    testCaseStore.activeTestCaseId = null

    // 加载 Auth 字段
    loadAuthFields(req.auth_type || 'none', req.auth_config || '{}')

    // 加载该接口历史 + 测试用例
    await historyStore.loadHistory(req.id)
    await testCaseStore.loadTestCases(req.id)
  } else {
    url.value = ''
    method.value = 'GET'
    bodyType.value = 'none'
    bodyContent.value = ''
    queryParams.value = []
    requestHeaders.value = []
    loadAuthFields('none', '{}')
    queryMode.value = 'table'
    headerMode.value = 'table'
    paramsDirty.value = false
    responseStore.clear()
  }
}, { immediate: true })

// ── URL 解析 ──────────────────────────────────────────────────
const parsedUrl = computed<ParsedUrl | null>(() => {
  if (!url.value) return null
  return parseUrl(url.value, method.value)
})

watch(parsedUrl, (p) => {
  if (!p) return
  const newVals: Record<string, string> = {}
  for (const { key, value } of p.pathParams) {
    newVals[key] = pathParamValues.value[key] ?? value
  }
  pathParamValues.value = newVals
})

// ── 构建发送时的真实 URL（path params 已替换）─────────────────
const resolvedUrl = computed(() => {
  if (!parsedUrl.value) return url.value
  return buildUrl(
    parsedUrl.value.pathTemplate,
    parsedUrl.value.pathParams.map(p => ({
      key: p.key,
      value: pathParamValues.value[p.key] ?? p.value,
    }))
  )
})

// ── URL {{var}} 高亮层 ────────────────────────────────────────
/** 将 URL 中的 {{variable}} 替换为带背景色 span，其余字符转义为 HTML 实体 */
const urlHighlighted = computed(() => {
  if (!url.value) return ''
  return escapeAndHighlight(url.value)
})

function escapeAndHighlight(s: string): string {
  // 按 {{...}} 分段，分别处理
  const parts = s.split(/({{[^{}]*}})/)
  return parts.map(part => {
    if (/^{{[^{}]*}}$/.test(part)) {
      // {{variable}} → 橙色徽章
      return `<mark class="url-var">${escapeHtmlChars(part)}</mark>`
    }
    return escapeHtmlChars(part)
  }).join('')
}

function escapeHtmlChars(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
}

// ── 辅助函数 ─────────────────────────────────────────────────
function addQueryParam() {
  queryParams.value.push({ key: '', value: '', enabled: true })
}

function addHeader() {
  requestHeaders.value.push({ key: '', value: '', enabled: true })
}

/** 将公共 Headers 模板中启用的条目批量写入编辑区（跳过 key 已存在的条目） */
function applyHeaderTemplate() {
  // 先确保 kv/json 模式的内容已同步到 requestHeaders
  if (headerMode.value === 'kv') requestHeaders.value = parseKvText(headerKvText.value)
  else if (headerMode.value === 'json') requestHeaders.value = parseJsonToParams(headerJsonText.value)

  const existingKeys = new Set(requestHeaders.value.map(h => h.key.toLowerCase()))
  const tplItems = headerTemplateStore.getEnabledItems()
  let added = 0
  for (const item of tplItems) {
    if (!existingKeys.has(item.key.toLowerCase())) {
      requestHeaders.value.push({ key: item.key, value: item.value, enabled: true })
      existingKeys.add(item.key.toLowerCase())
      added++
    }
  }
  // 若当前是 kv/json 模式，同步回文本框
  if (headerMode.value === 'kv') headerKvText.value = toKvText(requestHeaders.value)
  else if (headerMode.value === 'json') headerJsonText.value = toJsonText(requestHeaders.value)
}

// ── 发送请求 ──────────────────────────────────────────────────
async function handleSend() {
  // 非表格模式时先同步内容到 source of truth（queryParams / requestHeaders）
  if (queryMode.value === 'kv') queryParams.value = parseKvText(queryKvText.value)
  else if (queryMode.value === 'json') queryParams.value = parseJsonToParams(queryJsonText.value)
  if (headerMode.value === 'kv') requestHeaders.value = parseKvText(headerKvText.value)
  else if (headerMode.value === 'json') requestHeaders.value = parseJsonToParams(headerJsonText.value)
  // form-data 序列化
  if (bodyType.value === 'form_data') syncFormData()

  const activeReq = requestStore.activeRequest
  if (!activeReq) return

  const pathParamList: ParamItem[] = parsedUrl.value?.pathParams.map(p => ({
    key: p.key,
    value: pathParamValues.value[p.key] ?? p.value,
    enabled: true,
  })) ?? []

  const resp = await responseStore.sendRequest(
    activeReq.id,
    {
      method: method.value,
      url: resolvedUrl.value,
      query_params: queryParams.value,
      headers: requestHeaders.value,
      body_type: bodyType.value,
      body: bodyContent.value,
      path_params: pathParamList,
      auth_type: authType.value,
      auth_config: (() => {
        if (authType.value === 'bearer') return JSON.stringify({ token: authBearer.value })
        if (authType.value === 'basic') return JSON.stringify({ username: authBasicUser.value, password: authBasicPass.value })
        if (authType.value === 'api_key') return JSON.stringify({ key: authApiKeyName.value, value: authApiKeyValue.value, in: authApiKeyIn.value })
        return '{}'
      })(),
    },
    envStore.activeEnvId,
    projectStore.currentProjectId,
  )

  // 发送成功后，把新 history 记录插入本地缓存（避免重新拉取）
  if (resp && resp.history_id) {
    historyStore.prependRecord(activeReq.id, {
      id: resp.history_id,
      request_id: activeReq.id,
      status_code: resp.status_code,
      response_time_ms: resp.elapsed_ms,
      request_snapshot: JSON.stringify({
        method: method.value,
        url: resolvedUrl.value,
        query_params: queryParams.value,
        headers: requestHeaders.value,
        body_type: bodyType.value,
        body: bodyContent.value,
        path_params: pathParamList,
      }),
      response_body: resp.body,
      is_truncated: resp.is_truncated ? 1 : 0,
      response_headers: JSON.stringify(resp.headers),
      created_at: new Date().toISOString(),
    })
  }

  // 静默关联测试用例（低干扰：在响应返回后执行，不阻塞发送）
  if (resp) {
    const cases = testCaseStore.getByRequestId(activeReq.id)
    if (cases.length === 0) {
      // 第一次 Send：自动创建「用例 1」并收藏（后端自动命名）
      const tc = await testCaseStore.createTestCase({
        requestId: activeReq.id,
        collectionId: activeReq.collection_id,
        name: '',
        method: method.value,
        url: resolvedUrl.value,
        headers: JSON.stringify(requestHeaders.value),
        params_: JSON.stringify(queryParams.value),
        bodyType: bodyType.value,
        body: bodyContent.value,
      })
      testCaseStore.activeTestCaseId = tc.id
    }
    // 响应返回后检测参数是否与激活用例一致
    checkParamsDirty()
  }
}

// ── 参数变更检测 ───────────────────────────────────────────────
const paramsDirty = ref(false)

function checkParamsDirty() {
  const activeId = testCaseStore.activeTestCaseId
  if (activeId === null) { paramsDirty.value = false; return }
  const cases = testCaseStore.getByRequestId(requestStore.activeRequest?.id ?? 0)
  const activeTc = cases.find(c => c.id === activeId)
  if (!activeTc) { paramsDirty.value = false; return }

  const sameMethod = (activeTc.method ?? method.value) === method.value
  const sameUrl = (activeTc.url ?? resolvedUrl.value) === resolvedUrl.value
  const sameHeaders = activeTc.headers === JSON.stringify(requestHeaders.value)
  const sameParams = activeTc.params === JSON.stringify(queryParams.value)
  const sameBodyType = (activeTc.body_type ?? bodyType.value) === bodyType.value
  const sameBody = (activeTc.body ?? bodyContent.value) === bodyContent.value

  paramsDirty.value = !(sameMethod && sameUrl && sameHeaders && sameParams && sameBodyType && sameBody)
}

// ── 用例操作 handlers ──────────────────────────────────────────
async function handleActivateTestCase(id: number) {
  const cases = testCaseStore.getByRequestId(requestStore.activeRequest?.id ?? 0)
  const tc = cases.find(c => c.id === id)
  if (!tc) return
  testCaseStore.activeTestCaseId = id
  if (tc.method) method.value = tc.method
  if (tc.url) url.value = tc.url
  if (tc.headers) { try { requestHeaders.value = JSON.parse(tc.headers) } catch {} }
  if (tc.params) { try { queryParams.value = JSON.parse(tc.params) } catch {} }
  if (tc.body_type) bodyType.value = tc.body_type
  if (tc.body !== null && tc.body !== undefined) bodyContent.value = tc.body
  // 切换用例后重置模式为 table，避免 KV/JSON 模式显示旧数据
  queryMode.value = 'table'
  headerMode.value = 'table'
  paramsDirty.value = false
}

async function handleSaveToActive() {
  const activeId = testCaseStore.activeTestCaseId
  if (!activeId) return
  await testCaseStore.updateTestCase(activeId, {
    method: method.value,
    url: resolvedUrl.value,
    headers: JSON.stringify(requestHeaders.value),
    params: JSON.stringify(queryParams.value),
    body_type: bodyType.value,
    body: bodyContent.value,
  })
  paramsDirty.value = false
}

async function handleSaveAsNew() {
  const activeReq = requestStore.activeRequest
  if (!activeReq) return
  const tc = await testCaseStore.createTestCase({
    requestId: activeReq.id,
    collectionId: activeReq.collection_id,
    name: '',
    method: method.value,
    url: resolvedUrl.value,
    headers: JSON.stringify(requestHeaders.value),
    params_: JSON.stringify(queryParams.value),
    bodyType: bodyType.value,
    body: bodyContent.value,
  })
  testCaseStore.activeTestCaseId = tc.id
  paramsDirty.value = false
}

async function handleToggleStar(id: number) {
  const cases = testCaseStore.getByRequestId(requestStore.activeRequest?.id ?? 0)
  const tc = cases.find(c => c.id === id)
  if (!tc) return
  await testCaseStore.updateTestCase(id, { starred: tc.starred === 1 ? 0 : 1 })
}

async function handleDeleteTestCase(id: number) {
  try {
    await testCaseStore.deleteTestCase(id)
  } catch (e) {
    window.alert(String(e))
  }
}

async function handleRenameTestCase(id: number, name: string) {
  await testCaseStore.updateTestCase(id, { name })
}

async function handleCreateTestCase(name: string) {
  const activeReq = requestStore.activeRequest
  if (!activeReq) return
  await testCaseStore.createTestCase({
    requestId: activeReq.id,
    collectionId: activeReq.collection_id,
    name,
    method: method.value,
    url: resolvedUrl.value,
    headers: JSON.stringify(requestHeaders.value),
    params_: JSON.stringify(queryParams.value),
    bodyType: bodyType.value,
    body: bodyContent.value,
  })
}

// ── 历史回填 ──────────────────────────────────────────────────
function handleRefill(snapshot: string) {
  try {
    const s = JSON.parse(snapshot)
    if (s.method) method.value = s.method
    if (s.url) url.value = s.url
    if (s.query_params) queryParams.value = s.query_params
    if (s.headers) requestHeaders.value = s.headers
    if (s.body_type) bodyType.value = s.body_type
    if ('body' in s) bodyContent.value = s.body ?? ''
  } catch {
    // snapshot 解析失败时静默忽略
  }
}

// ── 压测 ──────────────────────────────────────────────────────
const stressStore = useStressStore()
const showStressConfig = ref(false)
const showStressResult = ref(false)

async function handleStartStress(config: StressConfig) {
  // 先同步 kv/json 模式内容到 source of truth
  if (queryMode.value === 'kv') queryParams.value = parseKvText(queryKvText.value)
  else if (queryMode.value === 'json') queryParams.value = parseJsonToParams(queryJsonText.value)
  if (headerMode.value === 'kv') requestHeaders.value = parseKvText(headerKvText.value)
  else if (headerMode.value === 'json') requestHeaders.value = parseJsonToParams(headerJsonText.value)
  if (bodyType.value === 'form_data') syncFormData()

  showStressResult.value = true

  await stressStore.startStress(
    {
      method: method.value,
      url: resolvedUrl.value,
      query_params: queryParams.value.map(p => ({ key: p.key, value: p.value, enabled: p.enabled })),
      headers: requestHeaders.value.map(h => ({ key: h.key, value: h.value, enabled: h.enabled })),
      body_type: bodyType.value,
      body: bodyContent.value,
      path_params: [],
    },
    config,
  )
}
</script>

<style scoped>
.main-panel {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
  background: var(--n-color, #fff);
}

.request-area {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
  padding: 12px 16px 0;
  min-height: 200px;
}

.url-bar {
  display: flex;
  gap: 8px;
  align-items: center;
  margin-bottom: 8px;
}

.request-tabs {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.divider {
  height: 4px;
  background: var(--n-border-color, #e0e0e6);
  cursor: row-resize;
  flex-shrink: 0;
}

.params-editor { padding: 8px 4px; overflow-y: auto; }
.params-section-label { font-size: 11px; font-weight: 600; color: var(--n-text-color-3, #999); padding: 4px 0 6px; text-transform: uppercase; letter-spacing: 0.5px; }
.param-row { display: flex; gap: 8px; align-items: center; margin-bottom: 6px; }
.tab-content-placeholder { padding: 20px 0; }

.params-mode-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 0 6px;
}
.mode-tabs {
  display: flex;
  gap: 0;
  border: 1px solid var(--n-border-color, #e0e0e6);
  border-radius: 4px;
  overflow: hidden;
}
.mode-tab {
  padding: 2px 8px;
  font-size: 11px;
  cursor: pointer;
  color: var(--n-text-color-3, #999);
  transition: background 0.1s, color 0.1s;
}
.mode-tab:hover { background: var(--n-item-color-hover, rgba(0,0,0,0.04)); color: var(--n-text-color, #333); }
.mode-tab.active { background: var(--n-primary-color, #18a058); color: #fff; }

/* ── URL {{var}} 高亮层 ── */
.url-input-wrap {
  flex: 1;
  position: relative;
  display: flex;
  align-items: center;
}

/* n-input 透明文字（字体/大小与高亮层完全一致），高亮层在下面透出来 */
.url-input-transparent :deep(.n-input__input-el) {
  color: transparent;
  caret-color: var(--n-text-color, #333);  /* 光标保持可见 */
  background: transparent;
  position: relative;
  z-index: 1;
}

.url-highlight-layer {
  position: absolute;
  left: 0;
  top: 0;
  right: 0;
  bottom: 0;
  padding: 0 12px;          /* 与 n-input medium 的内边距对齐 */
  display: flex;
  align-items: center;
  font-size: 14px;           /* 与 n-input medium 字号一致 */
  font-family: inherit;
  line-height: 1;
  pointer-events: none;     /* 不拦截鼠标事件 */
  overflow: hidden;
  white-space: pre;
  color: var(--n-text-color, #333);
  z-index: 0;
}

/* {{variable}} 标记样式 */
.url-highlight-layer :deep(.url-var) {
  background: rgba(250, 140, 22, 0.18);
  color: #d46b08;
  border-radius: 3px;
  padding: 1px 2px;
  font-style: normal;
}
</style>
