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
        <n-input
          v-model:value="url"
          placeholder="输入请求 URL，如 https://api.example.com/users"
          size="medium"
          style="flex: 1"
          @keyup.enter="handleSend"
        />
        <n-button
          type="primary"
          size="medium"
          style="flex-shrink: 0; min-width: 72px"
          :loading="responseStore.loading"
          @click="handleSend"
        >
          发 送
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

            <!-- Query Params（可编辑）-->
            <div class="params-section-label">Query Params</div>
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
          </div>
        </n-tab-pane>

        <n-tab-pane name="headers" tab="Headers">
          <div class="params-editor">
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
            </n-radio-group>
            <n-input
              v-if="bodyType !== 'none'"
              v-model:value="bodyContent"
              type="textarea"
              :rows="8"
              placeholder="请求体内容"
              style="font-family: monospace; font-size: 12px"
            />
          </div>
        </n-tab-pane>

        <n-tab-pane name="auth" tab="Auth">
          <div class="tab-content-placeholder">
            <n-empty description="Auth 配置（M4 实现）" size="small" />
          </div>
        </n-tab-pane>
      </n-tabs>
    </div>

    <!-- 分隔线 -->
    <div class="divider" />

    <!-- 下半：响应区 -->
    <ResponsePanel @refill="handleRefill" />
  </main>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import {
  NSelect, NInput, NButton, NTabs, NTabPane, NEmpty,
  NTag, NDivider, NCheckbox, NRadioGroup, NRadioButton,
} from 'naive-ui'
import { parseUrl, buildUrl } from '../../utils/urlParser'
import { useRequestStore } from '../../stores/request'
import { useResponseStore } from '../../stores/response'
import { useHistoryStore } from '../../stores/history'
import { useEnvironmentStore } from '../../stores/environment'
import { useProjectStore } from '../../stores/project'
import ResponsePanel from '../response/ResponsePanel.vue'
import type { ParamItem, ParsedUrl } from '../../types'

const requestStore = useRequestStore()
const responseStore = useResponseStore()
const historyStore = useHistoryStore()
const envStore = useEnvironmentStore()
const projectStore = useProjectStore()

// ── 请求编辑区状态 ────────────────────────────────────────────
const method = ref('GET')
const url = ref('')
const pathParamValues = ref<Record<string, string>>({})
const queryParams = ref<ParamItem[]>([])
const requestHeaders = ref<ParamItem[]>([])
const bodyType = ref('none')
const bodyContent = ref('')

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

    // 解析存储的 params/headers JSON
    try { queryParams.value = JSON.parse(req.params) } catch { queryParams.value = [] }
    try { requestHeaders.value = JSON.parse(req.headers) } catch { requestHeaders.value = [] }

    // 加载该接口历史
    await historyStore.loadHistory(req.id)
  } else {
    url.value = ''
    method.value = 'GET'
    bodyType.value = 'none'
    bodyContent.value = ''
    queryParams.value = []
    requestHeaders.value = []
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

// ── 辅助函数 ─────────────────────────────────────────────────
function addQueryParam() {
  queryParams.value.push({ key: '', value: '', enabled: true })
}

function addHeader() {
  requestHeaders.value.push({ key: '', value: '', enabled: true })
}

// ── 发送请求 ──────────────────────────────────────────────────
async function handleSend() {
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
</style>
