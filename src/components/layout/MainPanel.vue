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
    <div class="request-area" :style="{ height: requestAreaHeight + 'px' }">
      <!-- URL 栏 -->
      <div class="url-bar">
        <div class="url-input-combo">
          <n-dropdown :options="methodOptions" trigger="click" @select="selectMethod">
            <div class="method-trigger" :style="{ color: methodColor }">
              {{ method }} <span style="font-size: 10px; margin-left: 2px">▾</span>
            </div>
          </n-dropdown>
          <n-tooltip v-if="envStore.activeEnv" trigger="hover">
            <template #trigger>
              <n-tag size="small" type="success" :bordered="false" style="margin: 0 4px; cursor: default">
                {{ envTagText }}
              </n-tag>
            </template>
            {{ envTooltipText }}
          </n-tooltip>
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
              @change="onUrlEdit"
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
          text
          size="small"
          style="flex-shrink: 0"
          title="回到上次保存版（丢弃未保存修改；本会话未保存过则为接口原始定义）"
          @click="restoreToSaved"
        >↺</n-button>
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
              <!-- 1.0.4：表头行（值紧随字段名） -->
              <div v-else class="param-row param-row--header">
                <span class="param-col-check" />
                <span class="param-col-key" @click="toggleSort('query')">
                  字段名 {{ sortIndicators.query }}
                </span>
                <span class="param-col-value" style="flex:1">值</span>
                <span class="param-col-type">类型</span>
                <span class="param-col-desc">描述</span>
                <span class="param-col-del" />
              </div>
              <div v-for="(q, idx) in sortedQueryParams" :key="rowKey(idx, q)" class="param-row-wrap">
                <ParamRow :item="q" :type-options="typeOptions" key-placeholder="Key" value-placeholder="Value"
                  @remove="removeQueryParam(q)"
                  @pick-dict="openDictPicker" />
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
              <!-- 1.0.4：表头行（值紧随字段名） -->
              <div v-else class="param-row param-row--header">
                <span class="param-col-check" />
                <span class="param-col-key" @click="toggleSort('header')">
                  Header 名 {{ sortIndicators.header }}
                </span>
                <span class="param-col-value" style="flex:1">值</span>
                <span class="param-col-type">类型</span>
                <span class="param-col-desc">描述</span>
                <span class="param-col-del" />
              </div>
              <div v-for="(h, idx) in sortedHeaders" :key="rowKey(idx, h)" class="param-row-wrap">
                <ParamRow :item="h" :type-options="typeOptions" key-placeholder="Header 名" value-placeholder="值"
                  @remove="removeHeader(h)"
                  @pick-dict="openDictPicker" />
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
                <!-- 1.0.4：表头行（值紧随字段名） -->
                <div v-else class="param-row param-row--header">
                  <span class="param-col-check" />
                  <span class="param-col-key" @click="toggleSort('urlencoded')">
                    字段名 {{ sortIndicators.urlencoded }}
                  </span>
                  <span class="param-col-value" style="flex:1">值</span>
                  <span class="param-col-type">类型</span>
                  <span class="param-col-desc">描述</span>
                  <span class="param-col-del" />
                </div>
                <div v-for="(f, idx) in sortedUrlencoded" :key="rowKey(idx, f)" class="param-row-wrap">
                  <ParamRow :item="f" :type-options="typeOptions" key-placeholder="字段名" value-placeholder="值"
                    @remove="removeUrlencodedField(f)"
                    @pick-dict="openDictPicker" />
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
              <!-- 1.0.4：表头行（值紧随字段名） -->
              <div v-else class="param-row param-row--header">
                <span class="param-col-check" />
                <span class="param-col-key" @click="toggleSort('formdata')">
                  字段名 {{ sortIndicators.formdata }}
                </span>
                <span class="param-col-value" style="flex:1">值</span>
                <span class="param-col-type">类型</span>
                <span class="param-col-desc">描述</span>
                <span class="param-col-del" />
              </div>
              <div v-for="(f, idx) in sortedFormData" :key="rowKey(idx, f)" class="param-row-wrap">
                <ParamRow :item="f" :type-options="typeOptions" key-placeholder="字段名" value-placeholder="值"
                  @remove="removeFormDataField(f)"
                  @pick-dict="openDictPicker" />
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

        <!-- 1.0.4 fix：压测 Tab —— 历史结果查看 / 报告下载常驻入口（原先只在压测结果弹窗里可看） -->
        <n-tab-pane name="stress" tab="压测" display-directive="show:lazy">
          <StressTab @start="(c, t) => handleStartStress(c, t, false)" />
        </n-tab-pane>
      </n-tabs>
    </div>

    <!-- 1.0.4：字典选择弹窗（描述列 📖） -->
    <!-- 1.0.4：字段名 ↔ 字典绑定弹窗（描述列 📖） -->
    <n-modal v-model:show="showDictPicker" :preset="'card'" title="字段绑定字典" style="width: 480px">
      <div class="dict-bind">
        <div class="dict-bind__hint">
          {{ dictPickerParam ? `给字段「${dictPickerParam.key}」选择绑定字典：项目规则影响同项目所有同名；仅当前接口只作用于本接口。` : '' }}
        </div>
        <div class="dict-bind__scope">
          <n-radio-group v-model:value="bindScope" size="small">
            <n-radio-button value="rule">项目规则</n-radio-button>
            <n-radio-button value="override">仅当前接口</n-radio-button>
          </n-radio-group>
          <span v-if="bindScope === 'override' && requestStore.activeRequestId == null" class="dict-bind__hint">
            （当前无激活接口）
          </span>
        </div>
        <div v-if="boundInfo" class="dict-bind__current">
          <span>{{ bindScope === 'override' ? '接口例外' : '当前绑定' }}：</span>
          <n-tag size="small" :bordered="false" :type="boundInfo.type">{{ boundInfo.text }}</n-tag>
          <!-- 可解除时才给按钮（绑定了/设置了例外） -->
          <n-button v-if="boundInfo.unbindable" size="tiny" quaternary @click="unbindFieldDict">
            {{ bindScope === 'override' ? '恢复项目规则' : '解除绑定' }}
          </n-button>
        </div>
        <n-input
          v-model:value="dictSearchText"
          size="small"
          clearable
          placeholder="搜索字典（编码/名称）"
          class="dict-bind__search"
        />
        <div v-for="d in filteredDicts" :key="d.id"
          :class="['dict-bind__item', d.id === boundInfo?.dictId && 'dict-bind__item-active']"
          @click="bindFieldDict(d.id)">
          <span class="dict-bind__code">{{ d.code }}</span>
          <span class="dict-bind__name">{{ d.name }}</span>
          <span class="dict-bind__count">{{ (dictionaryStore.itemsMap[d.id] ?? []).length }} 项</span>
        </div>
        <div v-if="dictionaryStore.dictionaries.length && !filteredDicts.length" class="dict-bind__empty">
          没有匹配「{{ dictSearchText }}」的字典
        </div>
        <div class="dict-bind__empty" v-if="!dictionaryStore.dictionaries.length">暂无字典，请先在左侧 📖 数据字典侧栏创建</div>
      </div>
    </n-modal>

    <!-- 1.0.5 A2：退出用例前三选一（编辑区参数已修改且未存用例）-->
    <n-modal v-model:show="showDeactivateModal" preset="card" title="退出用例" style="width: 440px">
      <div class="deactivate-modal__tip">
        当前参数与用例「{{ activeCaseName }}」不同。退出后编辑区将回到接口原始参数，未保存的修改不会进入该用例。
      </div>
      <template #footer>
        <div class="deactivate-modal__actions">
          <n-button size="small" @click="cancelDeactivate">取消</n-button>
          <n-button size="small" quaternary type="warning" @click="confirmDeactivateDiscard">丢弃修改</n-button>
          <n-button size="small" type="primary" @click="confirmDeactivateSave">保存到用例</n-button>
        </div>
      </template>
    </n-modal>

    <!-- 分栏拖拽分隔条 -->
    <ResizableSplitter
      direction="vertical"
      :default-size="requestAreaHeight"
      :min-size="200"
      storage-key="layout.requestAreaHeight"
      @resize="(h: number) => requestAreaHeight = h"
    />

    <!-- 用例选择栏（Send 后出现，低干扰）-->
    <TestCaseBar
      :test-cases="testCaseStore.getByRequestId(requestStore.activeRequest?.id ?? 0)"
      :active-id="testCaseStore.activeTestCaseId"
      :params-dirty="paramsDirty"
      @activate="handleActivateTestCase"
      @deactivate="handleDeactivateTestCase"
      @create="handleCreateTestCase"
      @rename="handleRenameTestCase"
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
  NTag, NDivider, NRadioGroup, NRadioButton, NRadio,
  NDropdown, NTooltip, NModal,
  useMessage, useDialog,
} from 'naive-ui'
import { parseUrl, buildUrl, resolveEffectiveUrl, hasUnresolvedPlaceholder } from '../../utils/urlParser'
import { buildCurl } from '../../utils/curlBuilder'
import { parseKvText, toKvText, parseJsonToParams, toJsonText } from '../../utils/paramParser'
import { copyText } from '../../utils/clipboard'
import { useRequestStore, type RequestDraft } from '../../stores/request'
import { useResponseStore } from '../../stores/response'
import { useHistoryStore } from '../../stores/history'
import { useEnvironmentStore } from '../../stores/environment'
import { useProjectStore } from '../../stores/project'
import { useTestCaseStore } from '../../stores/testCase'
import { useStressStore } from '../../stores/stress'
import { useHeaderTemplateStore } from '../../stores/headerTemplate'
import { useDictionaryStore } from '../../stores/dictionary'
import { useTabStore } from '../../stores/tab'
import { useThemeStore } from '../../stores/theme'
import TabBar from './TabBar.vue'
import ResponsePanel from '../response/ResponsePanel.vue'
import TestCaseBar from '../testcase/TestCaseBar.vue'
import TestCaseManager from '../testcase/TestCaseManager.vue'
import StressConfigModal from '../stress/StressConfigModal.vue'
import StressResultPanel from '../stress/StressResultPanel.vue'
import StressTab from '../stress/StressTab.vue'
import ResizableSplitter from '../common/ResizableSplitter.vue'
import ParamRow from '../io/ParamRow.vue'
import type { ApiRequest, ParamItem, ParsedUrl, StressConfig } from '../../types'

type ParamMode = 'table' | 'kv' | 'json'

// 请求区高度（拖拽调节，上下分栏）
const requestAreaHeight = ref(
  Number(localStorage.getItem('layout.requestAreaHeight') ?? 400)
)

// ── Query Params 模式 ──────────────────────────────────────────
const queryMode = ref<ParamMode>('table')
const queryKvText = ref('')
const queryJsonText = ref('')

function switchQueryMode(newMode: ParamMode) {
  // 先把当前模式内容同步到 queryParams（table 是 source of truth）
  if (queryMode.value === 'kv') {
    queryParams.value = parseKvText(queryKvText.value, queryParams.value)
  } else if (queryMode.value === 'json') {
    queryParams.value = parseJsonToParams(queryJsonText.value, queryParams.value)
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
    requestHeaders.value = parseKvText(headerKvText.value, requestHeaders.value)
  } else if (headerMode.value === 'json') {
    requestHeaders.value = parseJsonToParams(headerJsonText.value, requestHeaders.value)
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
// 注意：不在此处动 testCaseId —— 由 .activeRequest watcher（loadRequest）统一
// 用 responseStore.setCurrent(req.id, null) 切回 raw 桶并清 activeTestCaseId。
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

// ── 1.0.4：参数表格 类型/描述/排序/字典引用 ───────────────────────
const dictionaryStore = useDictionaryStore()

const typeOptions = [
  { label: 'string', value: 'string' },
  { label: 'number', value: 'number' },
  { label: 'boolean', value: 'boolean' },
  { label: 'array', value: 'array' },
  { label: 'object', value: 'object' },
  { label: 'null', value: 'null' },
]

// 排序状态：target = 哪张表在排序；dir = 升降序
const sortTarget = ref<'query' | 'header' | 'urlencoded' | 'formdata' | null>(null)
const sortDir = ref<'asc' | 'desc'>('asc')

function toggleSort(target: 'query' | 'header' | 'urlencoded' | 'formdata') {
  if (sortTarget.value === target) {
    sortDir.value = sortDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    sortTarget.value = target
    sortDir.value = 'asc'
  }
}

const sortIndicators = computed(() => ({
  query: sortTarget.value === 'query' ? (sortDir.value === 'asc' ? '▲' : '▼') : '',
  header: sortTarget.value === 'header' ? (sortDir.value === 'asc' ? '▲' : '▼') : '',
  urlencoded: sortTarget.value === 'urlencoded' ? (sortDir.value === 'asc' ? '▲' : '▼') : '',
  formdata: sortTarget.value === 'formdata' ? (sortDir.value === 'asc' ? '▲' : '▼') : '',
}))

function sortedList(list: ParamItem[], target: 'query' | 'header' | 'urlencoded' | 'formdata'): ParamItem[] {
  if (sortTarget.value !== target) return list
  const sorted = [...list]
  sorted.sort((a, b) => {
    const r = a.key.localeCompare(b.key)
    return sortDir.value === 'asc' ? r : -r
  })
  return sorted
}
const sortedQueryParams = computed(() => sortedList(queryParams.value, 'query'))
const sortedHeaders = computed(() => sortedList(requestHeaders.value, 'header'))
const sortedUrlencoded = computed(() => sortedList(urlencodedParams.value, 'urlencoded'))
const sortedFormData = computed(() => sortedList(formDataParams.value, 'formdata'))

// 行 key 带接口作用域：切接口后强制重建行节点，杜绝 Vue 复用上一接口的行 DOM 导致残留
// 参数行稳定身份：WeakMap 给每个行对象分配自增 id，用作 v-for :key。
// 不能用「idx + key」当 key —— 一改字段名 key 就变，行被销毁重建、输入框失焦。
// WeakMap 映射不落库（持久化 JSON 不含该字段），切接口深拷贝产生新对象 → 自动分配新 id。
const paramRowIds = new WeakMap<object, number>()
let paramRowIdSeq = 1
function rowKey(_idx: number, p: ParamItem): string {
  let id = paramRowIds.get(p)
  if (id == null) {
    id = paramRowIdSeq++
    paramRowIds.set(p, id)
  }
  return `${requestStore.activeRequestId ?? 0}-row-${id}`
}

// ── 1.0.4 fix：用例切换不丢「类型/描述」 ──────────────────────
// 类型/描述属于接口定义；用例快照（旧数据）往往没有这些字段，切用例时若直接用
// 用例快照覆盖编辑区，类型/描述会消失。这里让用例缺失（空/undefined）的元数据
// 由接口参数同 key 补全；用例自己写过的仍然保留。
type MetaSource = Map<string, Pick<ParamItem, 'type' | 'description'>>

function metaFromRaw(jsonStr: string | null | undefined): MetaSource {
  try {
    const arr: ParamItem[] = JSON.parse(jsonStr || '[]')
    return new Map(arr.map(p => [p.key, p]))
  } catch {
    return new Map()
  }
}

function mergeParamMeta(list: ParamItem[], base: MetaSource): ParamItem[] {
  if (base.size === 0) return list
  return list.map(p => {
    const b = base.get(p.key)
    if (!b) return p
    const type = p.type == null || p.type === '' ? b.type : p.type
    const description = p.description == null || p.description === '' ? b.description : p.description
    if (type === p.type && description === p.description) return p
    return { ...p, type, description }
  })
}

/** 去掉 type/description/字典引用后比较 JSON 数组，用于 dirty 判定（元数据不参与） */
function stripMeta(json: string | null | undefined): string {
  // null/undefined 统一视为空数组，避免 "null" === "[]" 的误判
  if (json == null || json === '') return '[]'
  try {
    const arr: ParamItem[] = JSON.parse(json)
    if (!Array.isArray(arr)) return json
    return JSON.stringify(arr.map(p => ({ key: p.key, value: p.value, enabled: p.enabled })))
  } catch {
    return json
  }
}

// ── 1.0.4 fix：form_urlencoded 的 body 存储升级为结构化 JSON 数组 ──
// 旧存储是 k=v 文本（URLSearchParams 编码），放不下 type/description/字典引用，
// 导致 URL-Encoded 表格的类型/描述切用例、重启即丢。
// 现改为：DB/用例里 body 存 JSON 数组（与 form_data 一致，含全部元数据）；
// 发送请求时才编码为 k=v（syncUrlencodedData）。旧 k=v 数据加载时自动回退解析。
function parseUrlencodedBody(body: string | null | undefined): ParamItem[] {
  const raw = body ?? ''
  // 优先：结构化 JSON 数组
  try {
    const arr = JSON.parse(raw)
    if (Array.isArray(arr)) {
      return arr
        .filter((x: unknown): x is Record<string, unknown> => Boolean(x) && typeof x === 'object')
        .map(x => ({
          key: String(x.key ?? ''),
          value: String(x.value ?? ''),
          enabled: x.enabled !== false,
          type: typeof x.type === 'string' ? x.type : undefined,
          description: typeof x.description === 'string' ? x.description : undefined,
          descriptionDictRef: typeof x.descriptionDictRef === 'number' ? x.descriptionDictRef : undefined,
        }))
    }
  } catch { /* 非 JSON → 走旧格式兜底 */ }
  // 旧格式：k=v 文本
  try {
    const sp = new URLSearchParams(raw)
    const params: ParamItem[] = []
    sp.forEach((value, key) => params.push({ key, value, enabled: true }))
    return params
  } catch {
    return []
  }
}

/** 当前编辑区 urlencoded 的结构化存储串（发送仍走 k=v，见 syncUrlencodedData） */
function urlencodedStorageBody(): string {
  return JSON.stringify(urlencodedParams.value)
}

// 1.0.4：字段名 ↔ 字典 绑定弹窗（规则由 store 集中维护）
const dictPickerParam = ref<ParamItem | null>(null)
const showDictPicker = ref(false)
const dictionaryStoreLoaded = ref(false)

// 1.0.4：绑定作用域（项目规则 / 仅当前接口）
const bindScope = ref<'rule' | 'override'>('rule')
// 绑定弹窗内字典搜索过滤（编码/名称）
const dictSearchText = ref('')
const filteredDicts = computed(() => {
  const kw = dictSearchText.value.trim().toLowerCase()
  const list = !kw
    ? dictionaryStore.dictionaries
    : dictionaryStore.dictionaries.filter(d =>
        d.code.toLowerCase().includes(kw) || d.name.toLowerCase().includes(kw))
  return [...list].sort((a, b) => a.code.localeCompare(b.code))
})

const ruleBoundDictId = computed(() =>
  dictPickerParam.value
    ? dictionaryStore.ruleDictIdForField(dictPickerParam.value.key)
    : null)

// 当前作用域下已绑定的字典 id（用于「再点一次取消绑定」）
const currentBoundIdForScope = computed<number | null>(() => {
  const q = dictPickerParam.value
  if (!q) return null
  if (bindScope.value === 'rule') return ruleBoundDictId.value
  const rid = requestStore.activeRequestId
  if (rid == null) return null
  const ov = dictionaryStore.fieldOverrides.find(o => o.request_id === rid && o.field_name === q.key)
  return ov ? ov.dictionary_id : null
})

// 当前作用域下的绑定展示信息
const boundInfo = computed<{ text: string; type: 'success' | 'default' | 'warning'; dictId: number | null; unbindable: boolean } | null>(() => {
  const q = dictPickerParam.value
  if (!q) return null
  const dictOf = (id: number | null) => (id != null ? dictionaryStore.dictById(id) : null)
  if (bindScope.value === 'rule') {
    const d = dictOf(ruleBoundDictId.value)
    return d
      ? { text: `${d.code}（${d.name}）`, type: 'success' as const, dictId: d.id, unbindable: true }
      : { text: '未绑定', type: 'default' as const, dictId: null, unbindable: false }
  }
  // override：仅当前接口的例外状态
  if (requestStore.activeRequestId == null) {
    return { text: '无激活接口', type: 'default' as const, dictId: null, unbindable: false }
  }
  const ov = dictionaryStore.fieldOverrides.find(o =>
    o.request_id === requestStore.activeRequestId && o.field_name === q.key)
  if (!ov) return { text: '未设置例外（跟随项目规则）', type: 'default' as const, dictId: null, unbindable: false }
  if (ov.dictionary_id == null) {
    return { text: '已解除（本接口不命中字典）', type: 'warning' as const, dictId: null, unbindable: true }
  }
  const d = dictOf(ov.dictionary_id)
  return d
    ? { text: `${d.code}（${d.name}）`, type: 'success' as const, dictId: d.id, unbindable: true }
    : { text: '字典已删除', type: 'warning' as const, dictId: null, unbindable: true }
})

// 1.0.4 fix：项目就绪/切换时预加载字段绑定规则（保证描述列命中展示就绪；
// interface 面板下 DictionarySidebar 不渲染，只有此处负责加载，保留该 watch）
watch(() => projectStore.currentProjectId, (pid) => {
  dictionaryStore.loadFieldBindings(pid).catch(() => {})
}, { immediate: true })

async function openDictPicker(q: ParamItem) {
  dictPickerParam.value = q
  bindScope.value = 'rule'
  dictSearchText.value = ''
  if (!dictionaryStoreLoaded.value) {
    try {
      await dictionaryStore.loadDictionaries(projectStore.currentProjectId)
      await dictionaryStore.loadFieldBindings(projectStore.currentProjectId)
    } catch (e) {
      console.error('[dict-picker] load failed:', e)
      // 加载失败但已有缓存 → 仍可弹窗；完全无数据 → 明确报错而非“没反应”
      if (dictionaryStore.dictionaries.length === 0) {
        message.error('加载数据字典失败：' + String(e))
        dictionaryStoreLoaded.value = true
        return
      }
    }
    dictionaryStoreLoaded.value = true
  }
  // 未配置字典：不弹空面板，引导去字典侧栏创建
  if (dictionaryStore.dictionaries.length === 0) {
    message.info('暂无字典，请先在左侧 📖 数据字典侧栏创建')
    return
  }
  showDictPicker.value = true
}

/** 绑定/换绑：按作用域登记（rule=项目规则 / override=仅当前接口例外）；再点已绑定字典 = 取消/恢复 */
async function bindFieldDict(dictionaryId: number) {
  const q = dictPickerParam.value
  const pid = projectStore.currentProjectId
  if (!q || pid == null) return
  // 再次点击已绑定的字典 → 按当前作用域取消绑定（解除项目规则 / 恢复项目规则）
  if (dictionaryId === currentBoundIdForScope.value) {
    await unbindFieldDict()
    return
  }
  try {
    if (bindScope.value === 'override') {
      const rid = requestStore.activeRequestId
      if (rid == null) { message.warning('当前没有激活接口，无法设置接口例外'); return }
      await dictionaryStore.setFieldOverride(pid, rid, q.key, dictionaryId)
      message.success(`当前接口字段「${q.key}」已换绑字典`)
    } else {
      await dictionaryStore.setFieldRule(pid, q.key, dictionaryId)
      message.success(`字段「${q.key}」已绑定字典`)
    }
    showDictPicker.value = false
  } catch (e) {
    message.error(String(e))
  }
}

/** 解除：rule 作用域移除项目规则；override 作用域移除接口例外（恢复跟随项目规则） */
async function unbindFieldDict() {
  const q = dictPickerParam.value
  const pid = projectStore.currentProjectId
  if (!q || pid == null) return
  try {
    if (bindScope.value === 'override') {
      const rid = requestStore.activeRequestId
      if (rid == null) return
      await dictionaryStore.removeFieldOverride(pid, rid, q.key)
      message.success(`已恢复「${q.key}」跟随项目规则`)
    } else {
      await dictionaryStore.removeFieldRule(pid, q.key)
      message.success(`已解除「${q.key}」的字典绑定`)
    }
    showDictPicker.value = false
  } catch (e) {
    message.error(String(e))
  }
}

// form-data KV 字段（bodyType === 'form_data' 时使用）
const formDataParams = ref<ParamItem[]>([])

function addFormDataField() {
  pushUndo()
  formDataParams.value.push({ key: '', value: '', enabled: true })
}

/** 当 bodyType 为 form_data 时，将 formDataParams 序列化到 bodyContent */
function syncFormData() {
  if (bodyType.value === 'form_data') {
    bodyContent.value = JSON.stringify(formDataParams.value)
  }
}

/**
 * 把 KV/JSON 文本模式的内容同步回权威的 ParamItem[]（表格数据源）。
 * 任何读取 queryParams / requestHeaders 的入口（发送/保存/cURL/压测）都必须先调，
 * 否则会漏掉用户在文本态里的改动。body 的序列化方式各入口不同，不在此处理。
 */
function syncKvJsonToParams() {
  if (queryMode.value === 'kv') queryParams.value = parseKvText(queryKvText.value, queryParams.value)
  else if (queryMode.value === 'json') queryParams.value = parseJsonToParams(queryJsonText.value, queryParams.value)
  if (headerMode.value === 'kv') requestHeaders.value = parseKvText(headerKvText.value, requestHeaders.value)
  else if (headerMode.value === 'json') requestHeaders.value = parseJsonToParams(headerJsonText.value, requestHeaders.value)
}

const urlencodedParams = ref<ParamItem[]>([])
const urlencodedMode = ref<'table' | 'kv'>('table')
const urlencodedKvText = ref('')

function addUrlencodedField() {
  pushUndo()
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

/** KV文本变更时，解析回 urlencodedParams（同 key 保留类型/描述/字典引用，1.0.4 fix） */
function syncUrlencodedFromKv(text: string) {
  const prev = urlencodedParams.value
  const params: ParamItem[] = []
  for (const line of text.split('\n')) {
    const trimmed = line.trim()
    if (!trimmed || trimmed.startsWith('#')) continue
    const colonIdx = trimmed.indexOf(':')
    if (colonIdx > 0) {
      const key = trimmed.substring(0, colonIdx).trim()
      const value = trimmed.substring(colonIdx + 1).trim()
      const src = prev.find(p => p.key === key)
      params.push(src
        ? { key, value, enabled: true, type: src.type, description: src.description }
        : { key, value, enabled: true })
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

/** 把当前 auth 子字段序列化为 auth_config JSON 字符串（落库 / cURL / 发送共用同一套规则） */
function buildAuthConfigStr(): string {
  if (authType.value === 'bearer') return JSON.stringify({ token: authBearer.value })
  if (authType.value === 'basic') return JSON.stringify({ username: authBasicUser.value, password: authBasicPass.value })
  if (authType.value === 'api_key') return JSON.stringify({ key: authApiKeyName.value, value: authApiKeyValue.value, in: authApiKeyIn.value })
  return '{}'
}

/** 切换 auth 类型时，从接口数据恢复字段（或重置） */
function onAuthTypeChange(val: string) {
  authType.value = val
  syncAuthConfig()
}

/** 把当前 auth 子字段合并为 auth_config JSON 并保存到接口 */
function syncAuthConfig() {
  const cfg = buildAuthConfigStr()
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

// ── 环境标签文本（名称，hover 显示 base URL）─────────────────────
const envTagText = computed(() => {
  const env = envStore.activeEnv
  if (!env) return ''
  return env.name
})

const envTooltipText = computed(() => {
  const env = envStore.activeEnv
  if (!env) return ''
  return env.base_url ? `【Base URL】${env.base_url}` : '未配置 Base URL'
})

// ── 接口修改 dirty 标记 ────────────────────────────────────────────
const requestDirty = ref(false)
// paramsDirty（当前编辑区参数与激活用例是否有差异）的 ref 必须先于
// watch(() => requestStore.activeRequest, ..., { immediate: true }) 声明，
// 否则 immediate 回调的 else 分支里 `paramsDirty.value = false` 会触发
// TDZ (Cannot access 'paramsDirty' before initialization)。
// 真正的 checkParamsDirty 函数定义仍保留在文件后半部 (就近 TestCase 相关代码)。
const paramsDirty = ref(false)

/** 归一化 JSON 字符串（parse→stringify），消除空白/键序差异后比较 */
function normJson(s: string | null | undefined): string {
  try { return JSON.stringify(JSON.parse(s || '[]')) } catch { return s || '' }
}

/** 当前编辑区落库字段的签名（与 update_request 的内容字段对齐） */
function editorSignature(): string {
  // form_data 的落库形态是 formDataParams 的 JSON 序列化（与 syncFormData 输出一致）
  const bodySig = bodyType.value === 'form_data'
    ? JSON.stringify(formDataParams.value)
    : bodyType.value === 'form_urlencoded'
      ? urlencodedStorageBody()
      : bodyContent.value
  return JSON.stringify([
    method.value,
    url.value,
    JSON.stringify(queryParams.value),
    JSON.stringify(requestHeaders.value),
    bodyType.value,
    bodySig,
  ])
}

/** 接口落库状态的签名（比较基准 = store 里的 api_requests 值） */
function storedSignature(req: ApiRequest): string {
  const bodySig = (req.body_type === 'form_data' || req.body_type === 'form_urlencoded')
    ? normJson(req.body)
    : (req.body || '')
  return JSON.stringify([
    req.method,
    req.url,
    normJson(req.params),
    normJson(req.headers),
    req.body_type || 'none',
    bodySig,
  ])
}

/** 标记当前激活接口的 dirty 状态。
 *  1.0.5 修：改为「签名比较」派生而非事件置位——切回原值（如 Body 类型点出去又点回来）
 *  时编辑区与落库值一致，必须清脏，否则出现「什么都没改却显示待保存」的误报。 */
function markRequestDirty() {
  if (isInitializing) return
  const req = requestStore.activeRequest
  if (req == null) return
  const dirty = editorSignature() !== storedSignature(req)
  requestDirty.value = dirty
  if (dirty) {
    requestStore.dirtyRequestIds.add(req.id)
  } else {
    requestStore.dirtyRequestIds.delete(req.id)
  }
}

// ── 1.0.4 fix：参数元数据（类型/描述/字典引用）自动落库 ─────────
// 只在「离开该接口」的瞬间落库一次，锁定离开的接口 id：
//  - 无定时窗口 → 不存在把 A 的数据写进 B 的任何可能（问题 1.0.4-2 根治）
//  - 离开即保存 → 重启不丢；不干预 Ctrl+S / dirty 语义
function flushPersist(id: number) {
  if (bodyType.value === 'form_data') syncFormData()
  // form_urlencoded 落库用结构化 JSON（含类型/描述），发送时才编码 k=v
  const persistBody = bodyType.value === 'form_urlencoded' ? urlencodedStorageBody() : bodyContent.value
  void requestStore.persistParams(id, {
    params: JSON.stringify(queryParams.value),
    headers: JSON.stringify(requestHeaders.value),
    body_type: bodyType.value,
    body: persistBody,
  }).catch(() => {
    // 静默失败：不打断编辑，下次 Ctrl+S 兜底
  })
}

// ── 编辑区快照 / 恢复（切接口与用例 toggle 共用）──────────────
// ⚠️ 1.0.4 fix：参数数组必须深拷贝（JSON round-trip）后再入草稿。
// 若浅拷贝（[...arr]），元素对象仍是共享引用 —— A 的参数对象与草稿里的
// 是同一个，来回切接口时 A 的类型/描述会经由此共享引用“透”到 B 相同位置参数
// （特征：B 该位没描述被 A 覆盖，有描述则不覆盖）。深拷贝后各接口草稿完全隔离。
const deepCopyParams = (arr: ParamItem[]): ParamItem[] => JSON.parse(JSON.stringify(arr ?? []))

/** 把当前编辑区状态快照为草稿结构（切走接口写 draftCache 与进入用例前快照共用） */
function snapshotEditor(): RequestDraft {
  return {
    method: method.value,
    url: url.value,
    pathParamValues: { ...pathParamValues.value },
    queryParams: deepCopyParams(queryParams.value),
    requestHeaders: deepCopyParams(requestHeaders.value),
    bodyType: bodyType.value,
    bodyContent: bodyContent.value,
    formDataParams: deepCopyParams(formDataParams.value),
    urlencodedParams: deepCopyParams(urlencodedParams.value),
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

/** 从草稿快照完整恢复编辑区（切接口 2a 与 deactivate 返回接口编辑区共用） */
function applyDraftToEditor(draft: RequestDraft) {
  // ⚠️ 1.0.4 fix：恢复也必须深拷贝 —— 直接赋草稿数组会把「另一个接口的草稿数组引用」
  // 当成本接口编辑区（若某条路径误把 A 的数据存进 B 的草稿）。深拷贝后编辑区对象
  // 永远独立，物理上与其它接口无关。此前的浅拷贝恢复是最大嫌疑。
  url.value = draft.url
  method.value = draft.method
  bodyType.value = draft.bodyType
  bodyContent.value = draft.bodyContent
  queryParams.value = deepCopyParams(draft.queryParams)
  requestHeaders.value = deepCopyParams(draft.requestHeaders)
  formDataParams.value = deepCopyParams(draft.formDataParams)
  urlencodedParams.value = deepCopyParams(draft.urlencodedParams)
  // UI 模式状态跟随草稿
  queryMode.value = draft.queryMode
  queryKvText.value = draft.queryKvText
  queryJsonText.value = draft.queryJsonText
  headerMode.value = draft.headerMode
  headerKvText.value = draft.headerKvText
  headerJsonText.value = draft.headerJsonText
  urlencodedMode.value = draft.urlencodedMode
  urlencodedKvText.value = draft.urlencodedKvText
  // pathParamValues：URL 已从草稿恢复，parsedUrl 重算时会基于同样的 URL
  // 产生同样的 key 集合，直接赋值即可完整还原 value。
  pathParamValues.value = { ...draft.pathParamValues }
}

/** 从接口定义（DB）初始化编辑区，清空所有临时调试状态（切接口 2b 与 deactivate 兜底共用） */
function applyRequestToEditor(req: ApiRequest) {
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
    urlencodedParams.value = parseUrlencodedBody(req.body)
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

// ── 监听激活接口变化，同步到编辑区 ───────────────────────────
let isInitializing = false

// ── 编辑撤销 / 重做（1.0.5）─────────────────────────────────
// 结构级撤销：对 RequestDraft（snapshotEditor 粒度）压栈；文本输入在 ParamRow 内直接
// 变异行对象，走浏览器原生撤销，不进栈。栈按接口分列，切接口保留各自历史，深度 20。
const HISTORY_MAX = 20
const editorHistories = ref<Record<number, { undo: RequestDraft[]; redo: RequestDraft[] }>>({})

/** 本接口的撤销/重做栈（不存在则惰性建） */
function activeHistory() {
  const id = requestStore.activeRequestId
  if (id == null) return null
  if (!editorHistories.value[id]) editorHistories.value[id] = { undo: [], redo: [] }
  return editorHistories.value[id]
}

/** 结构变更前调用：快照当前编辑态入撤销栈、清空重做栈 */
function pushUndo() {
  const h = activeHistory()
  if (!h) return
  h.undo.push(snapshotEditor())
  if (h.undo.length > HISTORY_MAX) h.undo.shift()
  h.redo = []
}

function undo() {
  const h = activeHistory()
  if (!h || !h.undo.length) return
  h.redo.push(snapshotEditor())
  const prev = h.undo.pop()!
  isInitializing = true
  try { applyDraftToEditor(prev) } finally { nextTick(() => { isInitializing = false }) }
}

function redo() {
  const h = activeHistory()
  if (!h || !h.redo.length) return
  h.undo.push(snapshotEditor())
  const next = h.redo.pop()!
  isInitializing = true
  try { applyDraftToEditor(next) } finally { nextTick(() => { isInitializing = false }) }
}

// URL 编辑完成（失焦/回车）记一步；method 选择同样记一步（Auth 不入栈，已即时落库）
function onUrlEdit() { pushUndo() }
function selectMethod(k: string | number) { pushUndo(); method.value = String(k) }

// ── 草稿恢复（1.0.5）：回到上次 Ctrl+S 版本 ─────────────────
// 仅 Ctrl+S/💾（handleSaveRequest）才记录快照 —— flushPersist 切走时静默落库不是
// 用户认定的「保存」语义。按接口存，切接口保留。
const savedSnapshots = ref<Record<number, RequestDraft>>({})

function restoreToSaved() {
  const req = requestStore.activeRequest
  if (!req) return
  // 有本会话 Ctrl+S 快照 → 回快照；没有（从未保存过）→ 回 DB 原始定义
  const snap = requestStore.activeRequestId != null ? savedSnapshots.value[requestStore.activeRequestId] : null
  isInitializing = true
  try {
    if (snap) applyDraftToEditor(snap)
    else applyRequestToEditor(req)
  } finally { nextTick(() => { isInitializing = false }) }
  message.info('已恢复到上次保存的版本')
}

// ── 结构删除操作（带撤销快照） ───────────────────────────────
function removeQueryParam(q: ParamItem) {
  pushUndo()
  const i = queryParams.value.indexOf(q)
  if (i >= 0) queryParams.value.splice(i, 1)
}
function removeHeader(h: ParamItem) {
  pushUndo()
  const i = requestHeaders.value.indexOf(h)
  if (i >= 0) requestHeaders.value.splice(i, 1)
}
function removeUrlencodedField(f: ParamItem) {
  pushUndo()
  const i = urlencodedParams.value.indexOf(f)
  if (i >= 0) urlencodedParams.value.splice(i, 1)
}
function removeFormDataField(f: ParamItem) {
  pushUndo()
  const i = formDataParams.value.indexOf(f)
  if (i >= 0) formDataParams.value.splice(i, 1)
}
watch(() => requestStore.activeRequest, async (req, oldReq) => {
  // 1. 保存旧接口的草稿
  //    注意：无条件保存，不依赖 requestDirty —— 因为 pathParamValues 等
  //    "临时调试字段" 变动不触发 dirty，但仍需跨 Tab 保留。
  //    dirty 标记的唯一职责是控制"保存按钮亮起 / Ctrl+S 触发落库"，
  //    与"切 Tab 时是否保留编辑态"完全解耦。
  if (oldReq) {
    // 1.0.5：切走接口时进入用例前的接口编辑态快照随之失效
    //（跨接口后 toggle 返回无意义；切回本接口走既有草稿/DB 恢复路径）
    preCaseEditorSnapshot.value = null
    requestStore.draftCache[oldReq.id] = snapshotEditor()
    // 1.0.4 fix：切走即落库，锁定离开的接口 oldReq.id。
    // 此刻 queryParams 等仍是 oldReq 的编辑区内容（尚未被新接口覆盖），
    // 落库目标明确为 oldReq.id —— 类型/描述只会写回本接口，绝不串到目标接口。
    flushPersist(oldReq.id)
  }

  isInitializing = true
  let isDraft = false

  if (req) {
    const draft = requestStore.draftCache[req.id]
    if (draft) {
      // 2a. 从草稿恢复：所有编辑状态完整还原（深拷贝语义见 applyDraftToEditor）
      applyDraftToEditor(draft)
      isDraft = true
    } else {
      // 2b. 从 DB 加载：按接口原始定义初始化，清空所有临时调试状态
      applyRequestToEditor(req)
    }

    paramsDirty.value = false
    testCaseStore.activeTestCaseId = null
    urlPreview.value = false        // 切接口总回到编辑模式

    // 加载 Auth 字段（Auth 即时入库，总是从 req 读取，不走 draftCache）
    loadAuthFields(req.auth_type || 'none', req.auth_config || '{}')

    // 1.0.4：切换接口 → 响应视图切到该接口的 "raw"（原始参数）桶
    responseStore.setCurrent(req.id, null)

    // 加载该接口历史 + 测试用例
    await historyStore.loadHistory(req.id, null)
    await testCaseStore.loadTestCases(req.id)
    // 1.0.5：loadTestCases 不再自动激活任何用例（口径乙 / Q10），故此处
    // activeTestCaseId 恒为 null —— 打开接口停在原始参数干净态，响应与 History
    // 都留在 raw 桶。用户显式点击用例后，handleActivateTestCase 才切桶。
    responseStore.setCurrent(req.id, testCaseStore.activeTestCaseId)
    // History 也跟随激活用例加载，避免切回后 History 与响应不同步
    await historyStore.loadHistory(req.id, testCaseStore.activeTestCaseId)
  } else {
    // 3. 无激活接口：全部清空
    responseStore.setCurrent(null, null)
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
 * 若 URL 含 {{base_url}} 占位符，先用环境变量替换，再交给 resolveEffectiveUrl 处理
 */
const effectiveUrl = computed(() => {
  let raw = resolvedUrl.value
  // 替换 {{base_url}} 占位符（支持带空格的形式）
  if (envStore.activeEnv?.base_url && /\{\{\s*base_url\s*\}\}/.test(raw)) {
    raw = raw.replace(/\{\{\s*base_url\s*\}\}/g, envStore.activeEnv.base_url)
  }
  return resolveEffectiveUrl(raw, envStore.activeEnv?.base_url)
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
  pushUndo()
  queryParams.value.push({ key: '', value: '', enabled: true })
}

function addHeader() {
  pushUndo()
  requestHeaders.value.push({ key: '', value: '', enabled: true })
}

/** 将公共 Headers 模板中启用的条目批量写入编辑区（跳过 key 已存在的条目） */
function applyHeaderTemplate() {
  // 先确保 kv/json 模式的内容已同步到 requestHeaders
  if (headerMode.value === 'kv') requestHeaders.value = parseKvText(headerKvText.value, requestHeaders.value)
  else if (headerMode.value === 'json') requestHeaders.value = parseJsonToParams(headerJsonText.value, requestHeaders.value)

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

  // 1.0.3 Bug Fix：URL 参数变更时同步更新 KV/JSON 文本模式的内容
  // 否则用户在 KV/JSON 模式下修改 URL 后切换到表格模式再切回来，会看到旧数据
  queryKvText.value = toKvText(queryParams.value)
  queryJsonText.value = toJsonText(queryParams.value)

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

// 1.0.3 Bug Fix：Body 5 种格式互相转换
//   textarea 类（raw_json / raw_text）共享 bodyContent
//   table 类（form_urlencoded / form_data）各自有 ParamItem[] 数组
//   URL Encoded 内部还有 KV 文本子模式，切换前需先同步子模式到 urlencodedParams
watch(bodyType, (newType, oldType) => {
  if (isInitializing) return

  // ===== Step 0：切出前先同步源模式数据 =====
  // URL Encoded 的 KV 文本子模式：先解析回 urlencodedParams
  if (oldType === 'form_urlencoded' && urlencodedMode.value === 'kv') {
    syncUrlencodedFromKv(urlencodedKvText.value)
  }

  // ===== Step 1：切换到 table 类 =====

  // textarea 类 → URL Encoded
  if ((oldType === 'raw_json' || oldType === 'raw_text') && newType === 'form_urlencoded') {
    const text = bodyContent.value.trim()
    if (!text) return
    const params = parseTextToParams(text, urlencodedParams.value)
    if (params.length > 0) {
      urlencodedParams.value = params
      // 同步更新 KV 文本模式（如果用户在 URL Encoded 面板切到过 KV 模式）
      if (urlencodedMode.value === 'kv') {
        urlencodedKvText.value = params.filter(f => f.key).map(f => `${f.key}: ${f.value}`).join('\n')
      }
    }
    return
  }

  // textarea 类 → Form Data
  if ((oldType === 'raw_json' || oldType === 'raw_text') && newType === 'form_data') {
    const text = bodyContent.value.trim()
    if (!text) return
    const params = parseTextToParams(text, formDataParams.value)
    if (params.length > 0) formDataParams.value = params
    return
  }

  // table → table ：URL Encoded ↔ Form Data
  if (oldType === 'form_urlencoded' && newType === 'form_data') {
    formDataParams.value = urlencodedParams.value.map(f => ({ ...f }))
    return
  }
  if (oldType === 'form_data' && newType === 'form_urlencoded') {
    urlencodedParams.value = formDataParams.value.map(f => ({ ...f }))
    if (urlencodedMode.value === 'kv') {
      urlencodedKvText.value = urlencodedParams.value.filter(f => f.key).map(f => `${f.key}: ${f.value}`).join('\n')
    }
    return
  }

  // ===== Step 2：切换到 textarea 类 =====

  // table 类 → textarea 类
  if ((oldType === 'form_urlencoded' || oldType === 'form_data') && (newType === 'raw_json' || newType === 'raw_text')) {
    const source = oldType === 'form_urlencoded' ? urlencodedParams.value : formDataParams.value
    const enabled = source.filter(f => f.enabled && f.key)
    if (enabled.length === 0) return
    if (newType === 'raw_json') {
      const obj: Record<string, string> = {}
      enabled.forEach(f => { obj[f.key] = f.value })
      bodyContent.value = JSON.stringify(obj, null, 2)
    } else {
      const sp = new URLSearchParams()
      enabled.forEach(f => sp.append(f.key, f.value))
      bodyContent.value = sp.toString()
    }
    return
  }

  // JSON ↔ Text：切换时自动转换格式
  if (oldType === 'raw_json' && newType === 'raw_text') {
    const text = bodyContent.value.trim()
    if (!text) return
    try {
      const obj = JSON.parse(text)
      if (typeof obj === 'object' && obj !== null && !Array.isArray(obj)) {
        const sp = new URLSearchParams()
        Object.entries(obj).forEach(([k, v]) => sp.append(k, String(v)))
        bodyContent.value = sp.toString()
      }
      // 数组或 null 不做转换，保留原 JSON
    } catch { /* 非法 JSON，保留原文 */ }
    return
  }
  if (oldType === 'raw_text' && newType === 'raw_json') {
    const text = bodyContent.value.trim()
    if (!text) return
    // 尝试 key=value 格式 → JSON 对象
    try {
      const sp = new URLSearchParams(text)
      const obj: Record<string, string> = {}
      let hasEntries = false
      sp.forEach((val, key) => { obj[key] = val; hasEntries = true })
      if (hasEntries) bodyContent.value = JSON.stringify(obj, null, 2)
    } catch { /* 解析失败，保留原文 */ }
    return
  }
})

/** 将文本（JSON 或 key=value）解析为 ParamItem[]，转换失败返回空数组。
 * @param prev 切出前的旧表格，同 key 保留 type/description/字典引用（1.0.4 fix） */
function parseTextToParams(text: string, prev?: ParamItem[]): ParamItem[] {
  const withMeta = (items: ParamItem[]): ParamItem[] => {
    if (!prev) return items
    return items.map(it => {
      const src = prev.find(p => p.key === it.key)
      return src ? { ...it, type: src.type, description: src.description, descriptionDictRef: src.descriptionDictRef } : it
    })
  }
  // 优先尝试 JSON parse
  try {
    const obj = JSON.parse(text)
    if (typeof obj === 'object' && obj !== null && !Array.isArray(obj)) {
      return withMeta(Object.entries(obj).map(([k, v]) => ({
        key: k, value: String(v), enabled: true,
      })))
    }
  } catch { /* 非 JSON，继续尝试 */ }

  // 尝试 URLSearchParams（k1=v1&k2=v2）
  try {
    const sp = new URLSearchParams(text)
    const params: ParamItem[] = []
    sp.forEach((val, key) => { params.push({ key, value: val, enabled: true }) })
    return withMeta(params)
  } catch { /* 解析失败 */ }

  return []
}

// 监听其他编辑区字段变化，标记接口 dirty
watch(method, markRequestDirty)
watch(bodyType, markRequestDirty)
watch(bodyContent, markRequestDirty)
watch(requestHeaders, markRequestDirty, { deep: true })

// KV文本/JSON模式下，文本内容变化时即时解析回 queryParams（从而触发 URL 同步）
watch(queryKvText, (text) => {
  if (queryMode.value !== 'kv' || isInitializing) return
  queryParams.value = parseKvText(text, queryParams.value)
})

watch(queryJsonText, (text) => {
  if (queryMode.value !== 'json' || isInitializing) return
  queryParams.value = parseJsonToParams(text, queryParams.value)
})

// ── 复制当前编辑区为 cURL ───────────────────────────────────────
// 与 Sidebar 的右键复制 cURL 区别：本入口使用当前 MainPanel 编辑区 ref（含未保存修改 +
// pathParamValues 已填值），输出的 cURL URL 是 effectiveUrl —— 占位符已替换 + 拼 base_url。
async function handleCopyAsCurl() {
  // 与 handleSend 保持一致：先同步非表格模式内容到权威数据源，避免漏 KV/JSON 文本态改动
  syncKvJsonToParams()

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
  const authConfigStr = buildAuthConfigStr()

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

  await copyText(curl)

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
  syncKvJsonToParams()
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
    testCaseStore.activeTestCaseId,
    {
      method: method.value,
      url: effectiveUrl.value,   // 自动拼接环境 base_url，避免相对路径报错
      query_params: queryParams.value,
      headers: requestHeaders.value,
      body_type: bodyType.value,
      body: bodyContent.value,
      path_params: pathParamList,
      auth_type: authType.value,
      auth_config: buildAuthConfigStr(),
    },
    envStore.activeEnvId,
    projectStore.currentProjectId,
  )

  // 发送成功后，把新 history 记录插入本地缓存（避免重新拉取）
  if (resp && resp.history_id) {
    historyStore.prependRecord(activeReq.id, testCaseStore.activeTestCaseId, {
      id: resp.history_id,
      request_id: activeReq.id,
      test_case_id: testCaseStore.activeTestCaseId,
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
      error_message: null,
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
        body: bodyType.value === 'form_urlencoded' ? urlencodedStorageBody() : bodyContent.value,
      })
      // 1.0.4：响应此前写入了 raw 桶（尚未有用例），迁移到新用例桶后切视图
      responseStore.setCurrent(activeReq.id, tc.id)
      responseStore.moveResponse(null, tc.id)
      testCaseStore.activeTestCaseId = tc.id
    } else {
      // 1.0.4：存在用例时，确保响应视图切到当前激活用例桶（若用户刚切了用例）
      responseStore.setCurrent(activeReq.id, testCaseStore.activeTestCaseId)
    }
    // 响应返回后检测参数是否与激活用例一致
    checkParamsDirty()
  }

  // 刷新用例历史（数据已在 send_request 时写入 request_history）
  const activeTestCaseId = testCaseStore.activeTestCaseId
  if (activeTestCaseId !== null) {
    try {
      await testCaseStore.loadHistory(activeTestCaseId)
      // 口径乙：后端已更新该用例 last_run_at，重拉列表让「最近使用」排序反映本次发送
      await testCaseStore.loadTestCases(activeReq.id)
    } catch (e) {
      console.warn('[testCase] loadHistory failed:', e)
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

  // 1.0.0 fix:类型/描述/字典引用是「定义级元数据，不参与「参数不同」判定，
  // 否则用例切换时补全元数据会误触发 dirty 提示
  const sameMethod = (activeTc.method ?? method.value) === method.value
  // URL 比较：只比较 path 部分，因为 query string 由 queryParams 单独管理
  // 激活用例时会根据 tc.params 重建 URL，导致 query string 顺序可能与 tc.url 不一致
  // 因此不能直接比较完整 URL 字符串，而应该分别比较 path 和 params
  const tcUrlPath = (activeTc.url ?? '').split('?')[0]
  const currentUrlPath = url.value.split('?')[0]
  const sameUrl = tcUrlPath === currentUrlPath
  const sameHeaders = stripMeta(activeTc.headers) === stripMeta(JSON.stringify(requestHeaders.value))
  const sameParams = stripMeta(activeTc.params) === stripMeta(JSON.stringify(queryParams.value))
  const sameBodyType = (activeTc.body_type ?? bodyType.value) === bodyType.value
  const sameBody = stripMeta(activeTc.body ?? '') === stripMeta(bodyContent.value)

  // 调试日志：帮助定位误报原因
  if (!(sameMethod && sameUrl && sameHeaders && sameParams && sameBodyType && sameBody)) {
    console.log('[checkParamsDirty] 检测到参数差异:', {
      sameMethod, sameUrl, sameHeaders, sameParams, sameBodyType, sameBody,
      tcMethod: activeTc.method, currentMethod: method.value,
      tcUrlPath, currentUrlPath,
      tcHeaders: stripMeta(activeTc.headers), currentHeaders: stripMeta(JSON.stringify(requestHeaders.value)),
      tcParams: stripMeta(activeTc.params), currentParams: stripMeta(JSON.stringify(queryParams.value)),
      tcBodyType: activeTc.body_type, currentBodyType: bodyType.value,
      tcBody: stripMeta(activeTc.body ?? ''), currentBody: stripMeta(bodyContent.value),
    })
  }

  paramsDirty.value = !(sameMethod && sameUrl && sameHeaders && sameParams && sameBodyType && sameBody)
}

// ── 用例操作 handlers ──────────────────────────────────────────
// ── 1.0.5：用例上下文 toggle（再次点击已激活用例 = 回到接口编辑区）──
// 进入用例前的接口编辑态快照：deactivate 时据此返回。
// 存独立 ref 而非 draftCache —— 切走接口的既有草稿语义保持不变，
// toggle 的返回路径不受用例参数污染。
const preCaseEditorSnapshot = ref<RequestDraft | null>(null)
const showDeactivateModal = ref(false)

const activeCaseName = computed(() => {
  const id = testCaseStore.activeTestCaseId
  if (id == null) return ''
  const cases = testCaseStore.getByRequestId(requestStore.activeRequest?.id ?? 0)
  return cases.find(c => c.id === id)?.name ?? ''
})

async function handleActivateTestCase(id: number) {
  const cases = testCaseStore.getByRequestId(requestStore.activeRequest?.id ?? 0)
  const tc = cases.find(c => c.id === id)
  if (!tc) return
  // 从接口态进入用例态时，快照当前接口编辑态，供 deactivate 返回（A+A2）
  if (testCaseStore.activeTestCaseId === null) {
    preCaseEditorSnapshot.value = snapshotEditor()
  }
  isInitializing = true
  testCaseStore.activeTestCaseId = id
  // 1.0.4 fix：用例快照缺的「类型/描述」用接口参数同 key 补全（避免切用例即丢失）
  const activeReq = requestStore.activeRequest
  const metaParamSrc = activeReq ? metaFromRaw(activeReq.params) : new Map<string, ParamItem>()
  const metaBodySrc = activeReq && (activeReq.body_type === 'form_data' || activeReq.body_type === 'form_urlencoded')
    ? metaFromRaw(activeReq.body)
    : new Map<string, ParamItem>()
  // 1.0.4：切用例 → 响应视图切到该用例桶；History 按用例重载
  if (requestStore.activeRequestId != null) {
    responseStore.setCurrent(requestStore.activeRequestId, id)
    historyStore.loadHistory(requestStore.activeRequestId, id).catch(() => {})
  }
  if (tc.method) method.value = tc.method

  if (tc.headers) { try { requestHeaders.value = JSON.parse(tc.headers) } catch {} }

  if (tc.params) {
    try {
      const params: ParamItem[] = JSON.parse(tc.params)
      queryParams.value = mergeParamMeta(params, metaParamSrc)
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
    try { formDataParams.value = mergeParamMeta(JSON.parse(tc.body || '[]'), metaBodySrc) } catch { formDataParams.value = [] }
  } else {
    formDataParams.value = []
  }

  if (tc.body_type === 'form_urlencoded') {
    urlencodedParams.value = mergeParamMeta(parseUrlencodedBody(tc.body), metaBodySrc)
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

// ── 1.0.5：退出用例上下文（A+A2：再次点击已激活用例）────────────
function handleDeactivateTestCase() {
  // A2：有未保存修改 → 三选一（保存到用例 / 丢弃 / 取消）
  if (paramsDirty.value) {
    showDeactivateModal.value = true
    return
  }
  deactivateToRaw()
}

function cancelDeactivate() {
  showDeactivateModal.value = false
}

async function confirmDeactivateSave() {
  showDeactivateModal.value = false
  try {
    await handleSaveToActive()
    deactivateToRaw()
  } catch (e) {
    // 保存失败则不退出，留在用例上下文，避免修改悬空丢失
    message.error(`保存到用例失败：${e}`)
  }
}

function confirmDeactivateDiscard() {
  showDeactivateModal.value = false
  deactivateToRaw()
}

/** 退出用例上下文：编辑区回到接口编辑态（进入用例前快照优先，兜底接口定义），
 *  清激活用例，响应/History 切回 raw 桶 */
function deactivateToRaw() {
  const reqId = requestStore.activeRequestId
  const activeReq = requestStore.activeRequest
  if (reqId == null || !activeReq) return
  isInitializing = true
  testCaseStore.activeTestCaseId = null
  const snap = preCaseEditorSnapshot.value
  if (snap) {
    applyDraftToEditor(snap)
  } else {
    // 快照缺失（异常路径）：退回接口定义
    applyRequestToEditor(activeReq)
  }
  preCaseEditorSnapshot.value = null
  // 1.0.4 语义：raw 桶 = 无激活用例的接口调试上下文
  responseStore.setCurrent(reqId, null)
  historyStore.loadHistory(reqId, null).catch(() => {})
  paramsDirty.value = false
  // 与 handleActivateTestCase 同款收尾：等 URL/参数同步完再放开 watcher
  nextTick(() => {
    isInitializing = false
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
    body: bodyType.value === 'form_urlencoded' ? urlencodedStorageBody() : bodyContent.value,
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
    body: bodyType.value === 'form_urlencoded' ? urlencodedStorageBody() : bodyContent.value,
  })
  // 1.0.4：保存为新用例后，把当前 raw 桶响应迁到新用例桶并切视图
  responseStore.setCurrent(activeReq.id, tc.id)
  responseStore.moveResponse(null, tc.id)
  testCaseStore.activeTestCaseId = tc.id
  paramsDirty.value = false
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
    body: bodyType.value === 'form_urlencoded' ? urlencodedStorageBody() : bodyContent.value,
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
      urlencodedParams.value = parseUrlencodedBody(s.body)
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
  syncKvJsonToParams()
  if (bodyType.value === 'form_data') syncFormData()
  // form_urlencoded 存储用结构化 JSON（含类型/描述），发送时才编码 k=v
  const saveBody = bodyType.value === 'form_urlencoded' ? urlencodedStorageBody() : bodyContent.value

  // 1.0.3 Bug Fix：保存前快照当前用例 Tab ID，防止保存后自动跳回第一个
  const preSaveActiveTestCaseId = testCaseStore.activeTestCaseId

  try {
    await requestStore.updateRequest(req.id, {
      method: method.value,
      url: url.value,
      params: JSON.stringify(queryParams.value),
      headers: JSON.stringify(requestHeaders.value),
      body_type: bodyType.value,
      body: saveBody,
    })
    // 清除本地草稿缓存（updateRequest 内部已清除 dirtyRequestIds）
    const newCache = { ...requestStore.draftCache }
    delete newCache[req.id]
    requestStore.draftCache = newCache
    requestDirty.value = false
    // 草稿恢复基准：记录本次 Ctrl+S 的编辑区快照（不透传 Auth，Auth 已即时落库）
    savedSnapshots.value[req.id] = snapshotEditor()
    // 短暂显示绿色已保存小点
    requestStore.markSaved(req.id)

    // 恢复用例 Tab 状态（防止 updateRequest 或其他连锁反应清空 activeTestCaseId）
    if (preSaveActiveTestCaseId !== null) {
      testCaseStore.activeTestCaseId = preSaveActiveTestCaseId
    }

    message.success('已保存')
  } catch (e) {
    message.error('保存失败: ' + String(e))
  }
}

// Ctrl+S 保存 / Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y 撤销重做快捷键
function isEditable(el: EventTarget | null): boolean {
  const t = el as HTMLElement | null
  return !!t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable)
}

function onKeyDown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === 's') {
    e.preventDefault()
    if (requestDirty.value) handleSaveRequest()
    return
  }
  const key = e.key.toLowerCase()
  const mod = e.ctrlKey || e.metaKey
  // 撤销/重做：聚焦输入框时交给原生撤销，不做应用级接管
  if (mod && key === 'z') {
    if (isEditable(e.target)) return
    e.preventDefault()
    if (e.shiftKey) redo()
    else undo()
    return
  }
  if (mod && key === 'y') {
    if (isEditable(e.target)) return
    e.preventDefault()
    redo()
  }
}
onMounted(() => document.addEventListener('keydown', onKeyDown))
onUnmounted(() => {
  document.removeEventListener('keydown', onKeyDown)
})

// ── 压测 ──────────────────────────────────────────────────────
const stressStore = useStressStore()
const showStressConfig = ref(false)
const showStressResult = ref(false)

async function handleStartStress(config: StressConfig, testCaseId: number | null, openResultModal = true) {
  // 先同步 kv/json 模式内容到 source of truth
  syncKvJsonToParams()
  if (bodyType.value === 'form_data') syncFormData()
  if (bodyType.value === 'form_urlencoded') syncUrlencodedData()

  // 压测 tab 内启动时不弹结果弹窗，直接在当前 tab 看实时进度
  if (openResultModal) showStressResult.value = true

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
        // 用例 URL 常含 {{base_url}}（test-gen 技能生成格式）。压测引擎不做变量替换
        // （正常发送在 Rust 侧 replace_variables 处理），此处必须预替换，否则 URL
        // 静默不可解析、全部请求网络失败。与 effectiveUrl 一致，相对路径再拼 base_url。
        const baseUrl = envStore.activeEnv?.base_url
        const rawUrl = tc.url
        const hadPlaceholder = /\{\{\s*base_url\s*\}\}/i.test(rawUrl)
        const urlAfterPlaceholder = rawUrl.replace(/\{\{\s*base_url\s*\}\}/gi, baseUrl ?? '')
        if (/^https?:\/\//i.test(urlAfterPlaceholder)) {
          stressUrl = urlAfterPlaceholder
        } else if (baseUrl) {
          const base = baseUrl.replace(/\/$/, '')
          const path = urlAfterPlaceholder.startsWith('/') ? urlAfterPlaceholder : `/${urlAfterPlaceholder}`
          stressUrl = `${base}${path}`
        } else {
          stressUrl = urlAfterPlaceholder
        }
        // 兜底：仍含未替换的 {{...}}，或占位符存在却无激活环境 → 提示而不是静默失败
        if (/\{\{/.test(stressUrl) || (hadPlaceholder && !baseUrl)) {
          message.warning('压测 URL 含未替换的 {{...}} 占位符（未激活环境？），请求可能全部失败')
        }
      }
      try { stressQueryParams = JSON.parse(tc.params) } catch {}
      try { stressHeaders = JSON.parse(tc.headers) } catch {}
      if (tc.body_type) stressBodyType = tc.body_type
      if (tc.body != null) stressBody = tc.body
    }
  }

  await stressStore.startStress(
    requestStore.activeRequest?.id ?? 0,
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
  overflow: hidden;
  padding: var(--spacing-md) var(--spacing-lg) 0;
  flex-shrink: 0;
}

.url-bar {
  display: flex;
  gap: var(--spacing-sm);
  align-items: center;
  margin-bottom: var(--spacing-xs);
}

.url-input-combo {
  display: flex;
  align-items: center;
  flex: 1;
  border: 1px solid var(--border-base);
  border-radius: var(--radius-sm);
  background-color: var(--bg-elevated);
  transition: border-color 0.3s;
  padding-left: var(--spacing-sm);
}
.url-input-combo:focus-within {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 2px var(--color-primary-soft);
}
.method-trigger {
  font-weight: 600;
  font-size: var(--font-size-base);
  cursor: pointer;
  padding: var(--spacing-xs);
  border-radius: var(--radius-sm);
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

.params-editor { padding: var(--spacing-sm) var(--spacing-xs); overflow-y: auto; flex: 1; }
.params-section-label { font-size: var(--font-size-sm); font-weight: 600; color: var(--text-tertiary); padding: var(--spacing-xs) 0 var(--spacing-sm); text-transform: uppercase; letter-spacing: 0.5px; }
.param-row { display: flex; gap: var(--spacing-sm); align-items: center; margin-bottom: var(--spacing-sm); }

/* 1.0.4：参数表格增强 —— 表头行 / 列宽 / 排序 */
.param-row--header {
  font-size: var(--font-size-sm);
  font-weight: 600;
  color: var(--text-tertiary);
  padding: 2px 0;
  user-select: none;
}
.param-row--header .param-col-key { cursor: pointer; }
.param-row--header .param-col-key:hover { color: var(--color-primary); }
.param-col-check { width: 16px; flex-shrink: 0; }
.param-col-key { width: 140px; flex-shrink: 0; }
.param-col-type { width: 100px; flex-shrink: 0; }
.param-col-desc { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.param-col-value { flex: 1; min-width: 0; }
.param-col-del { width: 22px; flex-shrink: 0; }
.tab-content-placeholder { padding: var(--spacing-lg) 0; }

.params-mode-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-xs) 0 var(--spacing-sm);
}
.mode-tabs {
  display: flex;
  gap: 0;
  border: 1px solid var(--border-base);
  border-radius: var(--radius-sm);
  overflow: hidden;
}
.mode-tab {
  padding: 2px var(--spacing-sm);
  font-size: var(--font-size-sm);
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

/* {{variable}} 标记样式 —— 使用警告色（橙）token 统一。
   关键：padding 必须为 0 —— 有横向 padding 时高亮层每个变量比输入框真实文本
   宽几像素，光标/选区在 input 内的位置会与上层高亮的视觉位置错位。 */
.url-highlight-layer :deep(.url-var) {
  background: rgba(250, 140, 22, 0.18);
  color: var(--color-warning);
  border-radius: 3px;
  padding: 0;
  font-style: normal;
}

/* 自动注入 Headers 提示栏 */
.auto-headers-tip {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--spacing-sm);
  padding: var(--spacing-xs) var(--spacing-sm);
  margin-bottom: var(--spacing-sm);
  background: var(--bg-surface);
  border-radius: var(--radius-sm);
  font-size: var(--font-size-sm);
  color: var(--text-tertiary);
}
.auto-headers-label {
  font-weight: 500;
  white-space: nowrap;
}
.auto-header-badge {
  background: var(--border-base);
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  padding: 1px var(--spacing-sm);
  font-family: monospace;
  font-size: var(--font-size-sm);
}

/* 1.0.4：字段绑定字典弹窗 */
.dict-bind {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
  max-height: 380px;
  overflow-y: auto;
  padding: var(--spacing-sm) 0;
}
.dict-bind__hint {
  font-size: var(--font-size-sm);
  color: var(--text-tertiary);
  line-height: 1.6;
}
.dict-bind__current {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}
.dict-bind__item {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  padding: var(--spacing-sm);
  border: 1px solid var(--border-base);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: background 0.1s, border-color 0.1s;
}
.dict-bind__item:hover { background: var(--bg-hover); }
.dict-bind__item-active { border-color: var(--color-primary); background: var(--bg-selected); }
.dict-bind__code {
  font-family: monospace;
  font-weight: 700;
  color: var(--color-primary);
}
.dict-bind__name {
  color: var(--text-primary);
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.dict-bind__count { font-size: var(--font-size-sm); color: var(--text-tertiary); }
.dict-bind__empty { color: var(--text-tertiary); font-size: var(--font-size-sm); padding: var(--spacing-sm) 0; }

/* 无 Tab 时的空白引导页 */
.tab-empty-guide {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}
/* 1.0.5 A2：退出用例三选一弹窗 */
.deactivate-modal__tip {
  font-size: var(--font-size-base);
  color: var(--text-secondary);
  line-height: 1.6;
}
.deactivate-modal__actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-sm);
}
</style>
