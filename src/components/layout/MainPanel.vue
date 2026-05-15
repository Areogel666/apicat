<template>
  <main class="main-panel">
    <!-- Tab 标题栏（有 Tab 时显示） -->
    <TabBar v-if="tabStore.openTabs.length > 0" />

    <!-- 无激活 Tab 时显示引导页 -->
    <div v-if="tabStore.activeRequestId === null" class="tab-empty-guide">
      <n-empty description="← 从左侧选择接口开始调试" />
    </div>

    <!-- 有激活 Tab 时显示请求编辑区 + 响应区 -->
    <template v-else>
    <!-- 上半：请求编辑区 -->
    <div class="request-area">
      <!-- URL 栏 -->
      <div class="url-bar">
        <div class="url-input-combo">
          <n-dropdown :options="methodOptions" trigger="click" @select="(k) => method = String(k)">
            <div class="method-trigger" :style="{ color: methodColor }">
              {{ method }} <span style="font-size: 10px; margin-left: 2px">▾</span>
            </div>
          </n-dropdown>
          <n-tag v-if="envStore.activeEnv" size="small" type="success" :bordered="false" style="margin: 0 4px; cursor: default">
            {{ envTagText }}
          </n-tag>
          <!-- URL 输入框 + {{var}} 高亮层（仅有变量时才渲染高亮覆盖层） -->
          <div class="url-input-wrap">
            <!-- 高亮覆盖层：仅当 URL 含 {{var}} 时渲染，否则直接显示文字避免重影 -->
            <div v-if="urlHasVars && !urlPreview" class="url-highlight-layer" aria-hidden="true" v-html="urlHighlighted" />
            <!-- 编辑模式：正常输入原始 URL（含占位符） -->
            <n-input
              v-if="!urlPreview"
              v-model:value="url"
              placeholder="输入请求 URL,如 https://api.example.com/users"
              size="medium"
              :class="['url-input-transparent', urlHasVars ? 'url-vars-mode' : '']"
              :bordered="false"
              style="background: transparent"
              @keyup.enter="handleSend"
            />
            <!-- 预览模式：只读显示真实发送 URL（占位符已替换 + base_url 已拼接） -->
            <n-input
              v-else
              :value="effectiveUrl"
              size="medium"
              readonly
              :bordered="false"
              style="background: transparent; color: var(--text-secondary); font-style: italic"
              placeholder="(无 URL)"
              :title="'真实发送 URL（只读预览，点眼睛图标切回编辑）'"
            />
          </div>
          <!-- 眼睛切换按钮：在原始 URL 与真实发送 URL 间切换显示 -->
          <n-button
            text
            size="small"
            style="margin: 0 4px; flex-shrink: 0"
            :title="urlPreview ? '切换回编辑模式' : '预览真实发送 URL（占位符替换 + base_url 拼接）'"
            @click="urlPreview = !urlPreview"
          >
            {{ urlPreview ? '✏️' : '👁' }}
          </n-button>
        </div>
        <n-button
          v-if="requestDirty"
          size="medium"
          type="warning"
          style="flex-shrink: 0"
          title="保存接口 (Ctrl+S)"
          @click="handleSaveRequest"
        >
          💾 保存
        </n-button>
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
        <n-button
          size="medium"
          style="flex-shrink: 0"
          :disabled="!url"
          title="复制为 cURL（占位符使用当前面板值替换）"
          @click="handleCopyAsCurl"
        >
          📋 cURL
        </n-button>
      </div>

      <!-- 请求配置 Tabs -->
      <n-tabs type="line" size="small" class="request-tabs">
        <n-tab-pane name="params" tab="Params">
          <div class="params-editor">
            <!-- Path Params（由 URL 自动提取）-
                 template mode (:id / {id}) ：key 可改名，value 填值后不改 URL，发请求/cURL 时替换；
                 literal mode (123/UUID/abc123) ：key 只读，value 与 URL 字面量段双向同步 -->
            <template v-if="parsedUrl && parsedUrl.pathParams.length > 0">
              <div class="params-section-label">Path Params</div>
              <div
                v-for="(p, idx) in parsedUrl.pathParams"
                :key="`pp-${idx}-${p.key}-${p.mode}`"
                class="param-row"
              >
                <!-- template mode：key 可改名，同步改 URL 中的 :xxx / {xxx} -->
                <n-input
                  v-if="p.mode === 'template'"
                  :value="p.key"
                  size="small"
                  style="width:140px; flex-shrink:0"
                  placeholder="{key}"
                  title="模板参数：改名将同步到 URL 中的占位符"
                  @change="(val: string) => onPathKeyChange(p.key, val)"
                />
                <!-- literal mode：key 为只读派生标签，不可改名 -->
                <n-tag
                  v-else
                  size="small"
                  type="info"
                  style="width:140px; text-align:center; flex-shrink:0"
                  :title="`字面量参数：URL 中该段为 &quot;${p.segment}&quot;，key 不可改名`"
                >
                  {{ p.key }}
                </n-tag>

                <n-input
                  :value="pathParamValues[p.key] ?? ''"
                  size="small"
                  style="flex:1"
                  :placeholder="p.mode === 'template' ? '值（发送时替换占位符）' : '值（与 URL 双向同步）'"
                  @update:value="(val: string) => onPathValueChange(p.key, val)"
                />
                <n-button
                  size="tiny"
                  quaternary
                  title="从 URL 中删除该路径参数段"
                  @click="removePathParam(p.key)"
                >
                  ✕
                </n-button>
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

            <!-- 自动注入的 Headers 提示（只读，灰色展示）-->
            <div v-if="autoHeaders.length" class="auto-headers-tip">
              <span class="auto-headers-label">自动注入（发送时追加）：</span>
              <span v-for="h in autoHeaders" :key="h.key" class="auto-header-badge">
                {{ h.key }}: {{ h.value }}
              </span>
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
              <n-radio-button value="form_urlencoded">URL Encoded</n-radio-button>
              <n-radio-button value="form_data">Form Data</n-radio-button>
            </n-radio-group>

            <!-- 文本类 body -->
            <n-input
              v-if="bodyType === 'raw_json' || bodyType === 'raw_text'"
              v-model:value="bodyContent"
              type="textarea"
              :rows="8"
              placeholder="请求体内容"
              style="font-family: monospace; font-size: 12px"
            />

            <!-- form-urlencoded：表格 + KV文本 双模式 -->
            <template v-if="bodyType === 'form_urlencoded'">
              <div class="params-mode-bar" style="margin-bottom:6px">
                <span class="params-section-label" style="margin-bottom:0">URL Encoded 字段</span>
                <div class="mode-tabs">
                  <span :class="['mode-tab', urlencodedMode==='table' && 'active']" @click="urlencodedMode='table'">表格</span>
                  <span :class="['mode-tab', urlencodedMode==='kv' && 'active']" @click="switchUrlencodedToKv">KV 文本</span>
                </div>
              </div>
              <!-- 表格模式 -->
              <template v-if="urlencodedMode === 'table'">
                <n-empty v-if="!urlencodedParams.length" description="暂无字段" size="small" />
                <div v-for="(f, idx) in urlencodedParams" :key="idx" class="param-row">
                  <n-checkbox v-model:checked="f.enabled" />
                  <n-input v-model:value="f.key" size="small" style="width:140px; flex-shrink:0" placeholder="字段名" />
                  <n-input v-model:value="f.value" size="small" style="flex:1" placeholder="值" />
                  <n-button size="tiny" quaternary @click="urlencodedParams.splice(idx, 1)">✕</n-button>
                </div>
                <n-button size="small" dashed style="margin-top:4px; width:100%" @click="addUrlencodedField">
                  + 添加字段
                </n-button>
              </template>
              <!-- KV文本模式 -->
              <n-input
                v-else-if="urlencodedMode === 'kv'"
                v-model:value="urlencodedKvText"
                type="textarea"
                :rows="7"
                placeholder="key: value（每行一条，# 开头为注释）"
                style="font-family: monospace; font-size: 12px; margin-top: 4px"
                @update:value="syncUrlencodedFromKv"
              />
            </template>

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
            <div v-if="authType === 'none'" style="color: var(--text-tertiary); font-size:13px; padding:8px 0">
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
              <div style="font-size:11px;color:var(--text-tertiary);padding:4px 0">
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
              <div style="font-size:11px;color:var(--text-tertiary);padding:4px 0">
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

        <!-- M3-C：用例管理 Tab（与 Params/Headers/Body/Auth 平级） -->
        <n-tab-pane name="testcases" tab="用例" display-directive="show:lazy">
          <TestCaseManager :request-id="requestStore.activeRequest?.id ?? 0" />
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
      :test-cases="testCaseStore.getByRequestId(requestStore.activeRequest?.id ?? 0)"
      @start="handleStartStress"
    />

    <!-- 压测结果弹窗 -->
    <StressResultPanel v-model:show="showStressResult" />
    </template><!-- end v-else (has active tab) -->
  </main>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from 'vue'
import {
  NSelect, NInput, NButton, NTabs, NTabPane, NEmpty,
  NTag, NDivider, NCheckbox, NRadioGroup, NRadioButton, NRadio,
  NDropdown,
  useMessage, useDialog,
} from 'naive-ui'
import { parseUrl, buildUrl, resolveEffectiveUrl, hasUnresolvedPlaceholder } from '../../utils/urlParser'
import { buildCurl } from '../../utils/curlBuilder'
import { parseKvText, toKvText, parseJsonToParams, toJsonText } from '../../utils/paramParser'
import { useRequestStore } from '../../stores/request'
import { useResponseStore } from '../../stores/response'
import { useHistoryStore } from '../../stores/history'
import { useEnvironmentStore } from '../../stores/environment'
import { useProjectStore } from '../../stores/project'
import { useTestCaseStore } from '../../stores/testCase'
import { useStressStore } from '../../stores/stress'
import { useHeaderTemplateStore } from '../../stores/headerTemplate'
import { useTabStore } from '../../stores/tab'
import { useThemeStore } from '../../stores/theme'
import TabBar from './TabBar.vue'
import ResponsePanel from '../response/ResponsePanel.vue'
import TestCaseBar from '../testcase/TestCaseBar.vue'
import TestCaseManager from '../testcase/TestCaseManager.vue'
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

/**
 * 自动注入的 Headers：根据 body 类型和 auth 类型推导，
 * 仅用于 UI 展示（灰色提示），实际注入在 Rust 侧或 handleSend 中完成
 */
const autoHeaders = computed<Array<{key: string; value: string}>>(() => {
  const result: Array<{key: string; value: string}> = []
  // Content-Type 根据 body 类型自动注入
  const userHasContentType = requestHeaders.value.some(
    h => h.enabled && h.key.toLowerCase() === 'content-type'
  )
  if (!userHasContentType) {
    if (bodyType.value === 'raw_json') {
      result.push({ key: 'Content-Type', value: 'application/json' })
    } else if (bodyType.value === 'form_urlencoded') {
      result.push({ key: 'Content-Type', value: 'application/x-www-form-urlencoded' })
    } else if (bodyType.value === 'form_data') {
      result.push({ key: 'Content-Type', value: 'multipart/form-data' })
    }
  }
  // Auth 自动注入（Bearer / Basic）
  if (authType.value === 'bearer' && authBearer.value) {
    result.push({ key: 'Authorization', value: `Bearer ${authBearer.value}` })
  } else if (authType.value === 'basic' && authBasicUser.value) {
    const b64 = btoa(`${authBasicUser.value}:${authBasicPass.value}`)
    result.push({ key: 'Authorization', value: `Basic ${b64}` })
  }
  return result
})

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
const tabStore = useTabStore()
const themeStore = useThemeStore()
const message = useMessage()
const dialog = useDialog()

// tabStore.activeRequestId 是 Tab 的唯一激活来源
// request/response store 的 activeRequestId 都跟随它，确保响应面板按接口分桶展示
watch(() => tabStore.activeRequestId, (id) => {
  requestStore.activeRequestId = id
  responseStore.activeRequestId = id
}, { immediate: true })

// ── 请求编辑区状态 ────────────────────────────────────────────
const method = ref('GET')
const url = ref('')
// URL 框显示模式：false = 编辑原始 URL（含占位符），true = 只读预览真实发送 URL
// 不进 draftCache（纯 UI 状态），每个接口切换时不需要保留
const urlPreview = ref(false)
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

const urlencodedParams = ref<ParamItem[]>([])
const urlencodedMode = ref<'table' | 'kv'>('table')
const urlencodedKvText = ref('')

function addUrlencodedField() {
  urlencodedParams.value.push({ key: '', value: '', enabled: true })
}

/** 切换到KV文本模式时，先把表格序列化为 key: value 文本 */
function switchUrlencodedToKv() {
  urlencodedKvText.value = urlencodedParams.value
    .filter(f => f.key)
    .map(f => `${f.key}: ${f.value}`)
    .join('\n')
  urlencodedMode.value = 'kv'
}

/** KV文本变更时，解析回 urlencodedParams */
function syncUrlencodedFromKv(text: string) {
  const params: ParamItem[] = []
  for (const line of text.split('\n')) {
    const trimmed = line.trim()
    if (!trimmed || trimmed.startsWith('#')) continue
    const colonIdx = trimmed.indexOf(':')
    if (colonIdx > 0) {
      params.push({ key: trimmed.substring(0, colonIdx).trim(), value: trimmed.substring(colonIdx + 1).trim(), enabled: true })
    } else {
      params.push({ key: trimmed, value: '', enabled: true })
    }
  }
  urlencodedParams.value = params
}

function syncUrlencodedData() {
  if (bodyType.value === 'form_urlencoded') {
    const enabledFields = urlencodedParams.value.filter(f => f.enabled && f.key)
    const sp = new URLSearchParams()
    enabledFields.forEach(f => sp.append(f.key, f.value))
    bodyContent.value = sp.toString()
  }
}

// draftCache 已提升到 requestStore，此处直接使用 requestStore.draftCache（Ref<Record<number, RequestDraft>>）

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
  { label: 'GET',     value: 'GET', key: 'GET' },
  { label: 'POST',    value: 'POST', key: 'POST' },
  { label: 'PUT',     value: 'PUT', key: 'PUT' },
  { label: 'DELETE',  value: 'DELETE', key: 'DELETE' },
  { label: 'PATCH',   value: 'PATCH', key: 'PATCH' },
  { label: 'HEAD',    value: 'HEAD', key: 'HEAD' },
  { label: 'OPTIONS', value: 'OPTIONS', key: 'OPTIONS' },
]

// HTTP 方法色：从 CSS 变量读取，跟随主题切换
// methodColor 依赖 themeStore.effectiveMode 触发重算
const methodTokenMap: Record<string, string> = {
  GET: '--method-get',
  POST: '--method-post',
  PUT: '--method-put',
  DELETE: '--method-delete',
  PATCH: '--method-patch',
  HEAD: '--method-head',
  OPTIONS: '--method-options',
}
const methodColor = computed(() => {
  void themeStore.effectiveMode  // 显式依赖，主题切换时重算
  const token = methodTokenMap[method.value] ?? '--method-options'
  const v = getComputedStyle(document.documentElement).getPropertyValue(token).trim()
  return v || '#8c8c8c'  // 极早期 token 未注入时的兜底
})

// ── 环境标签文本（显示 baseURL(环境名) 格式）─────────────────────
const envTagText = computed(() => {
  const env = envStore.activeEnv
  if (!env) return ''
  if (env.base_url) return `${env.base_url}(${env.name})`
  return env.name
})

// ── 接口修改 dirty 标记 ────────────────────────────────────────────
const requestDirty = ref(false)
// paramsDirty（当前编辑区参数与激活用例是否有差异）的 ref 必须先于
// watch(() => requestStore.activeRequest, ..., { immediate: true }) 声明，
// 否则 immediate 回调的 else 分支里 `paramsDirty.value = false` 会触发
// TDZ (Cannot access 'paramsDirty' before initialization)。
// 真正的 checkParamsDirty 函数定义仍保留在文件后半部 (就近 TestCase 相关代码)。
const paramsDirty = ref(false)

/** 标记当前激活接口为有未保存修改状态 */
function markRequestDirty() {
  if (isInitializing) return
  const reqId = requestStore.activeRequest?.id
  if (reqId == null) return
  requestDirty.value = true
  // 用 Set 替换赋值触发 Vue 3 响应式（直接 add/delete 不触发）
  const newSet = new Set(requestStore.dirtyRequestIds)
  newSet.add(reqId)
  requestStore.dirtyRequestIds = newSet
}

// ── 监听激活接口变化，同步到编辑区 ───────────────────────────
let isInitializing = false
watch(() => requestStore.activeRequest, async (req, oldReq) => {
  // 1. 保存旧接口的草稿
  //    注意：无条件保存，不依赖 requestDirty —— 因为 pathParamValues 等
  //    "临时调试字段" 变动不触发 dirty，但仍需跨 Tab 保留。
  //    dirty 标记的唯一职责是控制"保存按钮亮起 / Ctrl+S 触发落库"，
  //    与"切 Tab 时是否保留编辑态"完全解耦。
  if (oldReq) {
    requestStore.draftCache[oldReq.id] = {
      method: method.value,
      url: url.value,
      pathParamValues: { ...pathParamValues.value },
      queryParams: [...queryParams.value],
      requestHeaders: [...requestHeaders.value],
      bodyType: bodyType.value,
      bodyContent: bodyContent.value,
      formDataParams: [...formDataParams.value],
      urlencodedParams: [...urlencodedParams.value],
      queryMode: queryMode.value,
      queryKvText: queryKvText.value,
      queryJsonText: queryJsonText.value,
      headerMode: headerMode.value,
      headerKvText: headerKvText.value,
      headerJsonText: headerJsonText.value,
      urlencodedMode: urlencodedMode.value,
      urlencodedKvText: urlencodedKvText.value,
    }
  }

  isInitializing = true
  let isDraft = false

  if (req) {
    const draft = requestStore.draftCache[req.id]
    if (draft) {
      // 2a. 从草稿恢复：所有编辑状态完整还原
      url.value = draft.url
      method.value = draft.method
      bodyType.value = draft.bodyType
      bodyContent.value = draft.bodyContent
      queryParams.value = draft.queryParams
      requestHeaders.value = draft.requestHeaders
      formDataParams.value = draft.formDataParams
      urlencodedParams.value = draft.urlencodedParams
      // UI 模式状态跟随草稿
      queryMode.value = draft.queryMode
      queryKvText.value = draft.queryKvText
      queryJsonText.value = draft.queryJsonText
      headerMode.value = draft.headerMode
      headerKvText.value = draft.headerKvText
      headerJsonText.value = draft.headerJsonText
      urlencodedMode.value = draft.urlencodedMode
      urlencodedKvText.value = draft.urlencodedKvText
      // pathParamValues：先清空再赋值，配合后续 watch(parsedUrl) 的合并逻辑。
      // 由于 URL 已从 draft 恢复，parsedUrl 重算时会基于同样的 URL 产生同样的 key 集合，
      // 此处直接赋值即可完整还原 value。
      pathParamValues.value = { ...draft.pathParamValues }
      isDraft = true
    } else {
      // 2b. 从 DB 加载：按接口原始定义初始化，清空所有临时调试状态
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

      if (req.body_type === 'form_urlencoded') {
        try {
          const sp = new URLSearchParams(req.body || '')
          const params: ParamItem[] = []
          sp.forEach((value, key) => params.push({ key, value, enabled: true }))
          urlencodedParams.value = params
        } catch { urlencodedParams.value = [] }
      } else {
        urlencodedParams.value = []
      }

      // 解析存储的 params/headers JSON
      try { queryParams.value = JSON.parse(req.params) } catch { queryParams.value = [] }
      try { requestHeaders.value = JSON.parse(req.headers) } catch { requestHeaders.value = [] }

      // 重置 UI 模式为默认 table（仅无草稿时）
      queryMode.value = 'table'
      queryKvText.value = ''
      queryJsonText.value = ''
      headerMode.value = 'table'
      headerKvText.value = ''
      headerJsonText.value = ''
      urlencodedMode.value = 'table'
      urlencodedKvText.value = ''
      // 关键：清空 pathParamValues，由后续 watch(parsedUrl) 根据新 URL 重建 key 集合
      pathParamValues.value = {}
    }

    paramsDirty.value = false
    testCaseStore.activeTestCaseId = null
    urlPreview.value = false        // 切接口总回到编辑模式

    // 加载 Auth 字段（Auth 即时入库，总是从 req 读取，不走 draftCache）
    loadAuthFields(req.auth_type || 'none', req.auth_config || '{}')

    // 加载该接口历史 + 测试用例
    await historyStore.loadHistory(req.id)
    await testCaseStore.loadTestCases(req.id)
  } else {
    // 3. 无激活接口：全部清空
    url.value = ''
    method.value = 'GET'
    bodyType.value = 'none'
    bodyContent.value = ''
    pathParamValues.value = {}
    queryParams.value = []
    requestHeaders.value = []
    formDataParams.value = []
    urlencodedParams.value = []
    loadAuthFields('none', '{}')
    queryMode.value = 'table'
    queryKvText.value = ''
    queryJsonText.value = ''
    headerMode.value = 'table'
    headerKvText.value = ''
    headerJsonText.value = ''
    urlencodedMode.value = 'table'
    urlencodedKvText.value = ''
    paramsDirty.value = false
    urlPreview.value = false
    // 注：不再主动 clear —— response 是基于 activeRequestId 的 computed 视图，
    // activeRequestId 为 null 时自动返回 null，无需干预桶本身（允许保留响应直到 Tab 关闭）。
  }

  // dirty 状态以 requestStore.dirtyRequestIds 为权威来源
  // （切 Tab 时无条件保存 draft 后，draft 存在 ≠ dirty，需用独立的 Set 追踪真正的脏状态）
  requestDirty.value = req ? requestStore.dirtyRequestIds.has(req.id) : false

  nextTick(() => {
    isInitializing = false
    if (isDraft) return

    // Bug1 修复：接口加载完毕后，同步 URL ↔ queryParams
    const reqUrl = url.value
    if (reqUrl) {
      const qsIdx = reqUrl.indexOf('?')
      if (qsIdx >= 0 && queryParams.value.length === 0) {
        // 方向 A：URL 有 query string 但 queryParams 为空 → 从 URL 解析
        const qs = reqUrl.substring(qsIdx + 1)
        const up = new URLSearchParams(qs)
        const parsed: ParamItem[] = []
        up.forEach((val, key) => parsed.push({ key, value: val, enabled: true }))
        if (parsed.length > 0) queryParams.value = parsed
      } else if (qsIdx < 0 && queryParams.value.length > 0) {
        // 方向 B：URL 无 query string 但 queryParams 有值（params 单独存储）→ 拼接到 URL
        // 这发生在 AI 生成的镜像接口上：url="/apm/intl/app", params=[25个参数]
        const enabledParams = queryParams.value.filter(p => p.enabled && p.key)
        if (enabledParams.length > 0) {
          const sp = new URLSearchParams()
          enabledParams.forEach(p => sp.append(p.key, p.value))
          url.value = `${reqUrl}?${sp.toString()}`
        }
      }
    }
  })
}, { immediate: true })

// ── URL 解析 ──────────────────────────────────────────────────
const parsedUrl = computed<ParsedUrl | null>(() => {
  if (!url.value) return null
  return parseUrl(url.value, method.value)
})

// 互锁标志：防止 URL ↔ pathParamValues 双向 watcher 循环触发
// - 面板改 value → 写入 URL → 禁用 parsedUrl 重算对 pathParamValues 的回写
// - URL 改 → 解析出新 pathParams → 同步 pathParamValues → 禁用 pathParamValues
//   watcher 对 URL 的回写
let syncingPathParamsFromUrl = false
let syncingUrlFromPathParams = false

watch(parsedUrl, (p) => {
  if (!p) return
  // 若正在反向 (面板→URL) 写入，parsedUrl 只是顺势重算，不要覆盖 pathParamValues
  if (syncingUrlFromPathParams) return

  const newVals: Record<string, string> = {}
  for (const { key, value } of p.pathParams) {
    // URL 是权威：按 URL 解析结果 value 覆盖；若 URL 中是占位符 (:id) 产出空 value，
    // 尝试保留面板中已有的用户输入（兼容"填 :id 再输值"的工作流）
    newVals[key] = value !== '' ? value : (pathParamValues.value[key] ?? '')
  }
  syncingPathParamsFromUrl = true
  pathParamValues.value = newVals
  // 微任务结束后释放锁
  nextTick(() => { syncingPathParamsFromUrl = false })
})

/**
 * 面板 value 修改时的处理（按 mode 分叉）。
 *
 * template mode（:id / {id}）：
 *   只更新 pathParamValues，URL 保持占位符形态不变；
 *   发送请求或复制 cURL 时由 buildUrl 把占位符替换为 value。
 *
 * literal mode（123 / UUID / abc123）：
 *   同步修改 URL 中对应字面量段，实现所见即所得的双向绑定。
 *   空值不回写 URL（避免 /download//orders 这种退化形态）。
 */
function onPathValueChange(key: string, newValue: string) {
  if (isInitializing || syncingPathParamsFromUrl) return

  const p = parsedUrl.value?.pathParams.find(pp => pp.key === key)
  if (!p) return

  // 先更新本地 pathParamValues（UI 即时反馈，两种 mode 都需要）
  const next = { ...pathParamValues.value }
  next[key] = newValue
  pathParamValues.value = next

  if (p.mode === 'template') {
    // 模板型：只在内存里存值，URL 不变
    // markRequestDirty 也不会触发（因为 URL 没变，但 pathParamValues 变了 ——
    // 这是否要标 dirty？值属于"调试态"，不落 DB，不标 dirty 是合理的）
    return
  }

  // literal mode：空值不回写（避免畸形 URL）
  if (newValue === '') return

  syncingUrlFromPathParams = true
  replacePathSegmentInUrl(key, newValue)
  nextTick(() => { syncingUrlFromPathParams = false })
}

/**
 * 面板 key 改名（仅 template mode 生效）。
 *
 * - 支持 "userId" / "{userId}" / ":userId" 三种输入形态，内部归一化为裸名
 * - 裸名校验：标识符规则（字母/下划线开头，后续 [\w]*）
 * - 去重检测：不允许改成和其他 param key 一样
 * - URL 同步：按原占位符风格保留
 *     :id  → :newBare
 *     {id} → {newBare}
 * - pathParamValues 中 value 从旧 key 迁移到新 key
 */
function onPathKeyChange(oldKey: string, rawNewKey: string) {
  if (isInitializing || syncingPathParamsFromUrl) return
  const trimmed = (rawNewKey ?? '').trim()
  if (!trimmed || trimmed === oldKey) return

  const target = parsedUrl.value?.pathParams.find(pp => pp.key === oldKey)
  if (!target || target.mode !== 'template') return

  // 归一化裸名：去掉可能的 { } 或前导 :
  let bare = trimmed
  if (bare.startsWith('{') && bare.endsWith('}')) bare = bare.slice(1, -1)
  if (bare.startsWith(':')) bare = bare.slice(1)
  if (!bare || !/^[A-Za-z_][\w]*$/.test(bare)) return

  // 去重检测
  const newKey = `{${bare}}`
  if (newKey === oldKey) return
  const others = parsedUrl.value!.pathParams.filter(pp => pp.key !== oldKey)
  if (others.some(pp => pp.key === newKey)) return

  // 迁移 value（在 URL 变更前先迁移，避免 watch(parsedUrl) 重建时丢失）
  const oldVal = pathParamValues.value[oldKey]
  const migrated = { ...pathParamValues.value }
  delete migrated[oldKey]
  if (oldVal !== undefined) migrated[newKey] = oldVal
  pathParamValues.value = migrated

  // 同步修改 URL：按原占位符风格保留（:id 风 / {id} 风）
  const newSegment = target.segment.startsWith(':') ? `:${bare}` : `{${bare}}`
  syncingUrlFromPathParams = true
  replacePathSegmentInUrl(oldKey, newSegment)
  nextTick(() => { syncingUrlFromPathParams = false })
}

/**
 * 反向同步：Path Params → URL
 *
 * 将给定 pathParam 的原始段在 url.value 中定位并替换为新内容。
 * 返回 false 表示定位失败（URL 已变动或无匹配段），此时调用方应放弃操作。
 *
 * 定位策略：pathParams 数组按 parseUrl 扫描顺序排列，所以 `targetKey` 在数组
 * 中的索引 = 在 URL path 中出现的第几个 pathParam 段。我们按"第 index+1 次
 * 出现 segment"在 URL 中做唯一替换,避免同名字面量段（如两个 123）被一起替换。
 */
function replacePathSegmentInUrl(targetKey: string, newSegment: string | null): boolean {
  const p = parsedUrl.value
  if (!p) return false
  const index = p.pathParams.findIndex(pp => pp.key === targetKey)
  if (index < 0) return false
  const target = p.pathParams[index]

  // 计算"目标段在原 URL 中是第几次出现"——pathParams 按扫描顺序排列，
  // 在同 segment 的 pathParam 中，当前项是第 N 个（N 从 0 开始）
  let occurrence = 0
  for (let i = 0; i < index; i++) {
    if (p.pathParams[i].segment === target.segment) occurrence++
  }

  // 在 url.value 中按 `/` 分段定位，跳过前 occurrence 次命中，替换第 occurrence+1 次
  const raw = url.value
  const qsIdx = raw.indexOf('?')
  const pathPart = qsIdx >= 0 ? raw.substring(0, qsIdx) : raw
  const queryPart = qsIdx >= 0 ? raw.substring(qsIdx) : ''

  // 分段（保留前导 `/` 的语义：以 `/` 切分后首项可能为空串表示绝对路径）
  const segs = pathPart.split('/')
  let hit = -1
  let skipped = 0
  for (let i = 0; i < segs.length; i++) {
    if (segs[i] === target.segment) {
      if (skipped === occurrence) { hit = i; break }
      skipped++
    }
  }
  if (hit < 0) return false

  if (newSegment === null) {
    // 删除：移除该段；注意保留周围的 `/` 结构
    segs.splice(hit, 1)
  } else {
    segs[hit] = newSegment
  }

  const newPath = segs.join('/')
  url.value = newPath + queryPart
  return true
}

/**
 * 从 URL 中删除指定 Path Params 对应的原始段。
 * pathParamValues 中对应的 key 会在后续 watch(parsedUrl) 里被自动清理。
 */
function removePathParam(targetKey: string) {
  syncingUrlFromPathParams = true
  replacePathSegmentInUrl(targetKey, null)
  nextTick(() => { syncingUrlFromPathParams = false })
}

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

/**
 * 发送前最终解析的完整 URL：
 * 调用 resolveEffectiveUrl 工具函数，与 Sidebar 的 cURL 复制共享同一套拼接规则。
 */
const effectiveUrl = computed(() => {
  return resolveEffectiveUrl(resolvedUrl.value, envStore.activeEnv?.base_url)
})

/** URL 中是否包含 {{var}} 变量，用于控制输入框文字是否透明（有变量才透明以展示高亮层） */
const urlHasVars = computed(() => /\{\{[^{}]*\}\}/.test(url.value))

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

let syncingFromUrl = false
let syncingFromParams = false

watch(url, () => {
  if (isInitializing || syncingFromParams) return
  // 用户修改 URL，标记接口为 dirty
  markRequestDirty()
  syncingFromUrl = true

  const qsIndex = url.value.indexOf('?')
  if (qsIndex >= 0) {
    const qs = url.value.substring(qsIndex + 1)
    const urlParams = new URLSearchParams(qs)
    const newParams: ParamItem[] = []
    urlParams.forEach((val, key) => {
      const existing = queryParams.value.find(p => p.key === key)
      newParams.push({
        key,
        value: val,
        enabled: existing ? existing.enabled : true
      })
    })
    const existingDisabled = queryParams.value.filter(p => !p.enabled)
    queryParams.value = [...newParams, ...existingDisabled]
  } else {
    queryParams.value = queryParams.value.filter(p => !p.enabled)
  }

  nextTick(() => { syncingFromUrl = false })
})

watch(queryParams, () => {
  if (isInitializing || syncingFromUrl) return
  markRequestDirty()
  syncingFromParams = true

  const enabledParams = queryParams.value.filter(p => p.enabled && p.key)
  const qsIndex = url.value.indexOf('?')
  const basePath = qsIndex >= 0 ? url.value.substring(0, qsIndex) : url.value

  if (enabledParams.length > 0) {
    const sp = new URLSearchParams()
    enabledParams.forEach(p => sp.append(p.key, p.value))
    url.value = `${basePath}?${sp.toString()}`
  } else {
    url.value = basePath
  }

  nextTick(() => { syncingFromParams = false })
}, { deep: true })

// 监听其他编辑区字段变化，标记接口 dirty
watch(method, markRequestDirty)
watch(bodyType, markRequestDirty)
watch(bodyContent, markRequestDirty)
watch(requestHeaders, markRequestDirty, { deep: true })

// KV文本/JSON模式下，文本内容变化时即时解析回 queryParams（从而触发 URL 同步）
watch(queryKvText, (text) => {
  if (queryMode.value !== 'kv' || isInitializing) return
  queryParams.value = parseKvText(text)
})

watch(queryJsonText, (text) => {
  if (queryMode.value !== 'json' || isInitializing) return
  queryParams.value = parseJsonToParams(text)
})

// ── 复制当前编辑区为 cURL ───────────────────────────────────────
// 与 Sidebar 的右键复制 cURL 区别：本入口使用当前 MainPanel 编辑区 ref（含未保存修改 +
// pathParamValues 已填值），输出的 cURL URL 是 effectiveUrl —— 占位符已替换 + 拼 base_url。
async function handleCopyAsCurl() {
  // 与 handleSend 保持一致：先同步非表格模式内容到权威数据源，避免漏 KV/JSON 文本态改动
  if (queryMode.value === 'kv') queryParams.value = parseKvText(queryKvText.value)
  else if (queryMode.value === 'json') queryParams.value = parseJsonToParams(queryJsonText.value)
  if (headerMode.value === 'kv') requestHeaders.value = parseKvText(headerKvText.value)
  else if (headerMode.value === 'json') requestHeaders.value = parseJsonToParams(headerJsonText.value)

  // 计算 body（form_data / form_urlencoded 需要序列化）
  let bodyStr = bodyContent.value
  if (bodyType.value === 'form_data') {
    bodyStr = JSON.stringify(formDataParams.value)
  } else if (bodyType.value === 'form_urlencoded') {
    const enabledFields = urlencodedParams.value.filter(f => f.enabled && f.key)
    const sp = new URLSearchParams()
    enabledFields.forEach(f => sp.append(f.key, f.value))
    bodyStr = sp.toString()
  }

  // 计算 auth_config（与 handleSend 保持同一套规则）
  const authConfigStr = (() => {
    if (authType.value === 'bearer') return JSON.stringify({ token: authBearer.value })
    if (authType.value === 'basic') return JSON.stringify({ username: authBasicUser.value, password: authBasicPass.value })
    if (authType.value === 'api_key') return JSON.stringify({ key: authApiKeyName.value, value: authApiKeyValue.value, in: authApiKeyIn.value })
    return '{}'
  })()

  const curl = buildCurl({
    method: method.value,
    url: effectiveUrl.value,    // 占位符已替换 + base_url 已拼接
    queryParams: queryParams.value,
    headers: requestHeaders.value,
    bodyType: bodyType.value,
    body: bodyStr,
    authType: authType.value,
    authConfig: authConfigStr,
  })

  try {
    await navigator.clipboard.writeText(curl)
  } catch {
    // Tauri 环境 clipboard 可能无权限，降级到 execCommand
    const ta = document.createElement('textarea')
    ta.value = curl
    document.body.appendChild(ta)
    ta.select()
    document.execCommand('copy')
    document.body.removeChild(ta)
  }

  // 编辑区场景占位符应已被 effectiveUrl 替换为空，若仍残留（用户未填值）给出提示
  if (hasUnresolvedPlaceholder(effectiveUrl.value)) {
    message.warning('cURL 包含未填值的占位符（:xxx / {xxx}），请在面板填值后再复制')
  } else {
    message.success('cURL 已复制到剪贴板')
  }
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
  if (bodyType.value === 'form_urlencoded') syncUrlencodedData()

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
      url: effectiveUrl.value,   // 自动拼接环境 base_url，避免相对路径报错
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
        url: effectiveUrl.value,   // 历史快照存完整 URL，便于回填
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

  // M3-C：写入用例历史。仅当当前有激活用例时记录（无激活 = 用户在用"原始参数"调试，不算用例执行）
  // 成功路径用 resp.status_code/elapsed_ms/body 摘要；失败路径用 responseStore.error
  // 同步在 sendRequest 后做，await 失败也不影响响应展示（store 内部已 try/catch）
  const activeTestCaseId = testCaseStore.activeTestCaseId
  if (activeTestCaseId !== null) {
    try {
      if (resp) {
        const preview = (resp.body ?? '').slice(0, 1024)  // 1KB 上限，避免 DB 膨胀
        await testCaseStore.recordHistory({
          testCaseId: activeTestCaseId,
          statusCode: resp.status_code,
          durationMs: resp.elapsed_ms,
          responsePreview: preview,
          errorMessage: null,
        })
      } else {
        // 网络层失败（DNS/超时/连接拒绝），无 status / 无 duration
        const errMsg = (responseStore.error ?? '请求失败').slice(0, 1024)
        await testCaseStore.recordHistory({
          testCaseId: activeTestCaseId,
          statusCode: null,
          durationMs: null,
          responsePreview: null,
          errorMessage: errMsg,
        })
      }
    } catch (e) {
      console.warn('[testCase] recordHistory failed:', e)
    }
  }
}

// ── 参数变更检测 ───────────────────────────────────────────────
// 注：`const paramsDirty = ref(false)` 已前移到 watch(requestStore.activeRequest)
// 之前，避免 immediate 回调 TDZ 报错。此处只保留 checkParamsDirty 函数。

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
  isInitializing = true
  testCaseStore.activeTestCaseId = id
  if (tc.method) method.value = tc.method

  if (tc.headers) { try { requestHeaders.value = JSON.parse(tc.headers) } catch {} }
  
  if (tc.params) {
    try {
      const params: ParamItem[] = JSON.parse(tc.params)
      queryParams.value = params
      // Rebuild URL: use tc.url as base, append enabled params as query string
      const enabledParams = params.filter(p => p.enabled && p.key)
      // tc.url=null 表示继承接口的 url，需 fallback 到 activeRequest.url
      const reqUrl = requestStore.activeRequest?.url ?? ''
      const rawBase = tc.url ?? reqUrl
      const basePath = rawBase.includes('?') ? rawBase.split('?')[0] : rawBase
      if (enabledParams.length > 0) {
        const sp = new URLSearchParams()
        enabledParams.forEach(p => sp.append(p.key, p.value))
        url.value = `${basePath}?${sp.toString()}`
      } else {
        url.value = basePath
      }
    } catch {
      queryParams.value = []
      url.value = tc.url ?? requestStore.activeRequest?.url ?? ''
    }
  } else {
    url.value = tc.url ?? requestStore.activeRequest?.url ?? ''
  }

  if (tc.body_type) bodyType.value = tc.body_type
  if (tc.body !== null && tc.body !== undefined) bodyContent.value = tc.body
  
  if (tc.body_type === 'form_data') {
    try { formDataParams.value = JSON.parse(tc.body || '[]') } catch { formDataParams.value = [] }
  } else {
    formDataParams.value = []
  }

  if (tc.body_type === 'form_urlencoded') {
    try {
      const sp = new URLSearchParams(tc.body || '')
      const params: ParamItem[] = []
      sp.forEach((value, key) => params.push({ key, value, enabled: true }))
      urlencodedParams.value = params
    } catch { urlencodedParams.value = [] }
  } else {
    urlencodedParams.value = []
  }

  // 切换用例后重置模式为 table，避免 KV/JSON 模式显示旧数据
  queryMode.value = 'table'
  headerMode.value = 'table'
  paramsDirty.value = false
  // Bug4 修复：先清除初始化标志，再防止 watch(queryParams) 反向覆写 URL
  nextTick(() => {
    isInitializing = false
    // queryParams 已从 tc.params 加载完毕，防止 watcher 反向重写 URL
    syncingFromParams = true
    nextTick(() => { syncingFromParams = false })
  })
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
  const cases = testCaseStore.getByRequestId(requestStore.activeRequest?.id ?? 0)
  const tc = cases.find(c => c.id === id)
  if (!tc) return
  dialog.warning({
    title: '确认删除',
    content: `确定要删除测试用例「${tc.name}」吗？此操作不可撤销！`,
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        await testCaseStore.deleteTestCase(id)
      } catch (e) {
        message.error(String(e))
      }
    },
  })
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
    isInitializing = true
    if (s.method) method.value = s.method
    if (s.url) url.value = s.url
    if (s.query_params) queryParams.value = s.query_params
    if (s.headers) requestHeaders.value = s.headers
    if (s.body_type) bodyType.value = s.body_type
    if ('body' in s) bodyContent.value = s.body ?? ''
    
    if (s.body_type === 'form_data') {
      try { formDataParams.value = JSON.parse(s.body || '[]') } catch { formDataParams.value = [] }
    } else {
      formDataParams.value = []
    }

    if (s.body_type === 'form_urlencoded') {
      try {
        const sp = new URLSearchParams(s.body || '')
        const params: ParamItem[] = []
        sp.forEach((value, key) => params.push({ key, value, enabled: true }))
        urlencodedParams.value = params
      } catch { urlencodedParams.value = [] }
    } else {
      urlencodedParams.value = []
    }

    nextTick(() => { isInitializing = false })
  } catch {
    // snapshot 解析失败时静默忽略
  }
}

// ── 接口保存（Bug2：暂存修改后保存到 DB）──────────────────────────
async function handleSaveRequest() {
  const req = requestStore.activeRequest
  if (!req) return
  // 非表格模式时先同步到 source of truth
  if (queryMode.value === 'kv') queryParams.value = parseKvText(queryKvText.value)
  else if (queryMode.value === 'json') queryParams.value = parseJsonToParams(queryJsonText.value)
  if (headerMode.value === 'kv') requestHeaders.value = parseKvText(headerKvText.value)
  else if (headerMode.value === 'json') requestHeaders.value = parseJsonToParams(headerJsonText.value)
  if (bodyType.value === 'form_data') syncFormData()
  if (bodyType.value === 'form_urlencoded') syncUrlencodedData()
  try {
    await requestStore.updateRequest(req.id, {
      method: method.value,
      url: url.value,
      params: JSON.stringify(queryParams.value),
      headers: JSON.stringify(requestHeaders.value),
      body_type: bodyType.value,
      body: bodyContent.value,
    })
    // 清除本地草稿缓存（updateRequest 内部已清除 dirtyRequestIds）
    const newCache = { ...requestStore.draftCache }
    delete newCache[req.id]
    requestStore.draftCache = newCache
    requestDirty.value = false
    // 短暂显示绿色已保存小点
    const savedSet = new Set(requestStore.savedRequestIds); savedSet.add(req.id); requestStore.savedRequestIds = savedSet
    setTimeout(() => {
      const s = new Set(requestStore.savedRequestIds); s.delete(req.id); requestStore.savedRequestIds = s
    }, 1500)
    message.success('已保存')
  } catch (e) {
    message.error('保存失败: ' + String(e))
  }
}

// Ctrl+S 快捷键保存
function onKeyDown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === 's') {
    e.preventDefault()
    if (requestDirty.value) handleSaveRequest()
  }
}
onMounted(() => document.addEventListener('keydown', onKeyDown))
onUnmounted(() => document.removeEventListener('keydown', onKeyDown))

// ── 压测 ──────────────────────────────────────────────────────
const stressStore = useStressStore()
const showStressConfig = ref(false)
const showStressResult = ref(false)

async function handleStartStress(config: StressConfig, testCaseId: number | null) {
  // 先同步 kv/json 模式内容到 source of truth
  if (queryMode.value === 'kv') queryParams.value = parseKvText(queryKvText.value)
  else if (queryMode.value === 'json') queryParams.value = parseJsonToParams(queryJsonText.value)
  if (headerMode.value === 'kv') requestHeaders.value = parseKvText(headerKvText.value)
  else if (headerMode.value === 'json') requestHeaders.value = parseJsonToParams(headerJsonText.value)
  if (bodyType.value === 'form_data') syncFormData()
  if (bodyType.value === 'form_urlencoded') syncUrlencodedData()

  showStressResult.value = true

  // 若选择了用例，使用用例的参数覆盖当前参数
  let stressMethod = method.value
  let stressUrl = effectiveUrl.value
  let stressQueryParams = queryParams.value.map(p => ({ key: p.key, value: p.value, enabled: p.enabled }))
  let stressHeaders = requestHeaders.value.map(h => ({ key: h.key, value: h.value, enabled: h.enabled }))
  let stressBodyType = bodyType.value
  let stressBody = bodyContent.value

  if (testCaseId !== null) {
    const cases = testCaseStore.getByRequestId(requestStore.activeRequest?.id ?? 0)
    const tc = cases.find(c => c.id === testCaseId)
    if (tc) {
      if (tc.method) stressMethod = tc.method
      if (tc.url) {
        // 和 effectiveUrl 一样：相对路径自动拼接激活环境的 base_url
        const rawUrl = tc.url
        if (/^https?:\/\//i.test(rawUrl)) {
          stressUrl = rawUrl
        } else {
          const baseUrl = envStore.activeEnv?.base_url
          if (baseUrl) {
            const base = baseUrl.replace(/\/$/, '')
            const path = rawUrl.startsWith('/') ? rawUrl : `/${rawUrl}`
            stressUrl = `${base}${path}`
          } else {
            stressUrl = rawUrl
          }
        }
      }
      try { stressQueryParams = JSON.parse(tc.params) } catch {}
      try { stressHeaders = JSON.parse(tc.headers) } catch {}
      if (tc.body_type) stressBodyType = tc.body_type
      if (tc.body != null) stressBody = tc.body
    }
  }

  await stressStore.startStress(
    {
      method: stressMethod,
      url: stressUrl,
      query_params: stressQueryParams,
      headers: stressHeaders,
      body_type: stressBodyType,
      body: stressBody,
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
  background: var(--bg-elevated);
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
  margin-bottom: 4px;
}

.url-input-combo {
  display: flex;
  align-items: center;
  flex: 1;
  border: 1px solid var(--border-base);
  border-radius: 3px;
  background-color: var(--bg-elevated);
  transition: border-color 0.3s;
  padding-left: 8px;
}
.url-input-combo:focus-within {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 2px var(--color-primary-soft);
}
.method-trigger {
  font-weight: 600;
  font-size: 13px;
  cursor: pointer;
  padding: 4px;
  border-radius: 3px;
  display: flex;
  align-items: center;
  gap: 2px;
}
.method-trigger:hover {
  background-color: var(--border-base);
}

.request-tabs {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

/* 穿透 Naive UI n-tabs 内部容器，使 tab-pane 参与 flex 布局并限定高度 */
.request-tabs :deep(.n-tabs-pane-wrapper),
.request-tabs :deep(.n-tab-pane) {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  height: 0; /* flex 子项需要 height: 0 才能被压缩到容器高度 */
}

.divider {
  height: 4px;
  background: var(--border-base);
  cursor: row-resize;
  flex-shrink: 0;
}

.params-editor { padding: 8px 4px; overflow-y: auto; flex: 1; }
.params-section-label { font-size: 11px; font-weight: 600; color: var(--text-tertiary); padding: 4px 0 6px; text-transform: uppercase; letter-spacing: 0.5px; }
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
  border: 1px solid var(--border-base);
  border-radius: 4px;
  overflow: hidden;
}
.mode-tab {
  padding: 2px 8px;
  font-size: 11px;
  cursor: pointer;
  color: var(--text-tertiary);
  transition: background 0.1s, color 0.1s;
}
.mode-tab:hover { background: var(--bg-hover); color: var(--text-primary); }
.mode-tab.active { background: var(--color-primary); color: #fff; }

/* ── URL {{var}} 高亮层 ── */
.url-input-wrap {
  flex: 1;
  position: relative;
  display: flex;
  align-items: center;
}

/* n-input 默认正常显示文字 */
.url-input-transparent :deep(.n-input__input-el) {
  background: transparent;
  position: relative;
  z-index: 1;
}

/* 有 {{var}} 变量时才透明（高亮层透出），无变量时正常显示文字颜色 */
.url-input-transparent.url-vars-mode :deep(.n-input__input-el) {
  color: transparent;
  caret-color: var(--text-primary);  /* 光标保持可见 */
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
  color: var(--text-primary);
  z-index: 0;
}

/* {{variable}} 标记样式 —— 使用警告色（橙）token 统一 */
.url-highlight-layer :deep(.url-var) {
  background: rgba(250, 140, 22, 0.18);
  color: var(--color-warning);
  border-radius: 3px;
  padding: 1px 2px;
  font-style: normal;
}

/* 自动注入 Headers 提示栏 */
.auto-headers-tip {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  padding: 4px 6px;
  margin-bottom: 6px;
  background: var(--bg-surface);
  border-radius: 4px;
  font-size: 11px;
  color: var(--text-tertiary);
}
.auto-headers-label {
  font-weight: 500;
  white-space: nowrap;
}
.auto-header-badge {
  background: var(--border-base);
  color: var(--text-secondary);
  border-radius: 3px;
  padding: 1px 6px;
  font-family: monospace;
  font-size: 11px;
}

/* 无 Tab 时的空白引导页 */
.tab-empty-guide {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
