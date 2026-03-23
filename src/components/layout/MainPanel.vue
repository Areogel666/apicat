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
        />
        <n-button type="primary" size="medium" style="flex-shrink: 0">
          发 送
        </n-button>
      </div>

      <!-- 请求配置 Tabs -->
      <n-tabs type="line" size="small" class="request-tabs">
        <n-tab-pane name="params" tab="Params">
          <div class="params-editor">
            <!-- Path Params（由 URL 自动提取，不可删除）-->
            <template v-if="parsedUrl && parsedUrl.pathParams.length > 0">
              <div class="params-section-label">Path Params</div>
              <div v-for="p in parsedUrl.pathParams" :key="p.key" class="param-row">
                <n-tag size="small" type="info" style="width:120px; text-align:center; flex-shrink:0">{{ p.key }}</n-tag>
                <n-input
                  v-model:value="pathParamValues[p.key]"
                  size="small"
                  style="flex:1"
                  placeholder="值"
                />
              </div>
              <n-divider style="margin: 8px 0" />
            </template>

            <!-- Query Params（从 URL 自动解析）-->
            <div class="params-section-label">Query Params</div>
            <n-empty v-if="!parsedUrl?.queryParams.length" description="暂无参数" size="small" />
            <div v-for="q in (parsedUrl?.queryParams ?? [])" :key="q.key" class="param-row">
              <n-input :value="q.key" size="small" style="width:140px; flex-shrink:0" placeholder="Key" readonly />
              <n-input :value="q.value" size="small" style="flex:1" placeholder="Value" />
            </div>
          </div>
        </n-tab-pane>
        <n-tab-pane name="headers" tab="Headers">
          <div class="tab-content-placeholder">
            <n-empty description="暂无 Headers" size="small" />
          </div>
        </n-tab-pane>
        <n-tab-pane name="body" tab="Body">
          <div class="tab-content-placeholder">
            <n-empty description="暂无 Body" size="small" />
          </div>
        </n-tab-pane>
        <n-tab-pane name="auth" tab="Auth">
          <div class="tab-content-placeholder">
            <n-empty description="暂无鉴权配置" size="small" />
          </div>
        </n-tab-pane>
      </n-tabs>
    </div>

    <!-- 分隔线（M2 实现拖拽调整高度） -->
    <div class="divider" />

    <!-- 下半：响应展示区 -->
    <div class="response-area">
      <div class="response-status-bar">
        <span class="status-label">响应</span>
        <!-- M3 起展示真实状态码/耗时/大小 -->
      </div>
      <n-tabs type="line" size="small" class="response-tabs">
        <n-tab-pane name="body" tab="Body (美化)">
          <div class="tab-content-placeholder response-placeholder">
            <n-empty description="发送请求后，响应内容将在这里显示" />
          </div>
        </n-tab-pane>
        <n-tab-pane name="headers" tab="Headers">
          <div class="tab-content-placeholder">
            <n-empty description="暂无响应 Headers" size="small" />
          </div>
        </n-tab-pane>
        <n-tab-pane name="history" tab="History">
          <div class="tab-content-placeholder">
            <n-empty description="暂无历史记录" size="small" />
          </div>
        </n-tab-pane>
      </n-tabs>
    </div>
  </main>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { NSelect, NInput, NButton, NTabs, NTabPane, NEmpty, NTag, NDivider } from 'naive-ui'
import { parseUrl } from '../../utils/urlParser'
import { useRequestStore } from '../../stores/request'
import type { ParsedUrl } from '../../types'

const requestStore = useRequestStore()

const method = ref('GET')
const methodOptions = [
  { label: 'GET',     value: 'GET' },
  { label: 'POST',    value: 'POST' },
  { label: 'PUT',     value: 'PUT' },
  { label: 'DELETE',  value: 'DELETE' },
  { label: 'PATCH',   value: 'PATCH' },
  { label: 'HEAD',    value: 'HEAD' },
  { label: 'OPTIONS', value: 'OPTIONS' },
]

const url = ref('')

// 监听激活接口变化，同步到编辑区
watch(() => requestStore.activeRequest, (req) => {
  if (req) {
    url.value = req.url
    method.value = req.method
  }
}, { immediate: true })

// URL 解析结果（实时响应式）
const parsedUrl = computed<ParsedUrl | null>(() => {
  if (!url.value) return null
  return parseUrl(url.value, method.value)
})

// Path Params 可编辑值（key → value）
const pathParamValues = ref<Record<string, string>>({})

// 当 parsedUrl 变化时，合并已有值（保留用户已填的值）
watch(parsedUrl, (p) => {
  if (!p) return
  const newVals: Record<string, string> = {}
  for (const { key, value } of p.pathParams) {
    newVals[key] = pathParamValues.value[key] ?? value
  }
  pathParamValues.value = newVals
})
</script>

<style scoped>
.main-panel {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
  background: var(--n-color, #fff);
}

/* ── 请求区 ── */
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

/* ── 分隔线 ── */
.divider {
  height: 4px;
  background: var(--n-border-color, #e0e0e6);
  cursor: row-resize;
  flex-shrink: 0;
}

/* ── 响应区 ── */
.response-area {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
  padding: 0 16px 12px;
  min-height: 180px;
}

.response-status-bar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 6px 0 2px;
  font-size: 12px;
  color: var(--n-text-color-3, #999);
  flex-shrink: 0;
}

.status-label {
  font-weight: 600;
  font-size: 13px;
  color: var(--n-text-color-2, #666);
}

.response-tabs {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.tab-content-placeholder {
  padding: 20px 0;
}

.params-editor { padding: 8px 4px; }
.params-section-label { font-size: 11px; font-weight: 600; color: var(--n-text-color-3, #999); padding: 4px 0 6px; text-transform: uppercase; letter-spacing: 0.5px; }
.param-row { display: flex; gap: 8px; align-items: center; margin-bottom: 6px; }

.response-placeholder {
  padding: 40px 0;
}
</style>
