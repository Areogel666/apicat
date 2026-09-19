// 与 Rust types.rs 保持一致的前端类型定义
//
// ⚠️ 无编译期机制防漂移，靠人工同步。改任一侧的字段（增/删/改名/改可空）
// 必须同时改另一侧，否则前端会读到 undefined 或 Rust 序列化出多余字段。
// 已发现的漂移史：Collection.updated_at（TS 多出，表里根本没有该列）、
// ApiRequest.description（Rust 有、TS 曾缺失）。

export interface Project {
  id: number
  name: string
  description: string | null
  docs_output_dir: string | null   // 1.0.5：doc-gen 技能的文档输出目录
  p95_threshold_ms: number | null  // 1.0.6：压测 P95 参考线（项目级默认）
  p99_threshold_ms: number | null  // 1.0.6：压测 P99 参考线（项目级默认）
  created_at: string
  updated_at: string
}

export interface Collection {
  id: number
  project_id: number
  parent_id: number | null
  name: string
  sort_order: number
  created_at: string
}

// ── 压测类型 ──────────────────────────────────────────────
export interface StressStats {
  total: number
  success: number
  failed: number
  success_rate: number   // 0.0 ~ 100.0（响应率：拿到响应的比例）
  avg_ms: number
  min_ms: number
  p50_ms: number
  p90_ms: number
  p95_ms: number
  p99_ms: number
  max_ms: number
  tps: number
  elapsed_sec: number
  done: boolean
  // 1.0.4 新增：耗时直方图（10 桶累计计数）+ 状态码分布 [status, count][]（0=网络错误）
  latency_hist?: number[]
  status_counts?: Array<[number, number]>
  // 1.0.5 新增：业务成功率（按期望状态码判定）。1.0.4 及以前的记录没有这几项
  biz_success?: number
  biz_success_rate?: number
  expect_status?: string
}

export interface StressConfig {
  concurrent: number     // 1 ~ 500
  mode: 'count' | 'duration'
  value: number          // 总请求数 或 持续秒数
  /** 期望状态码表达式，如 `2xx` / `200` / `2xx,3xx`（1.0.5） */
  expect_status: string
  /** P95 参考线阈值 ms，null = 用默认 500（1.0.6） */
  p95_threshold_ms?: number | null
  /** P99 参考线阈值 ms，null = 用默认 1000（1.0.6） */
  p99_threshold_ms?: number | null
}

// 折线图数据点（每次 stress://progress 推送时追加一条）
export interface StressChartPoint {
  time: number           // 相对压测开始的秒数
  tps: number
  avg_ms: number
  p95_ms: number
}

export interface ApiRequest {
  id: number
  collection_id: number
  name: string
  method: string
  url: string
  params: string       // JSON 数组文本
  headers: string      // JSON 数组文本
  body_type: string
  body: string
  auth_type: string
  auth_config: string
  description: string  // 1.0.4 新增：与 types.rs 的 ApiRequest.description 对应
  p95_threshold_ms: number | null  // 1.0.6：压测 P95 参考线（接口级覆盖）
  p99_threshold_ms: number | null  // 1.0.6：压测 P99 参考线（接口级覆盖）
  sort_order: number
  created_at: string
  updated_at: string
}

// URL 解析结果
export interface ParsedUrl {
  displayName: string       // "/apm/intl/download"（不含 method，避免树节点拼接重复）
  pathTemplate: string      // "/apm/intl/download/{id}"
  // segment：该 pathParam 在原始 URL 中的段文本（如 ":id" / "123" / "abc-uuid"）
  // 用于反向定位并修改原始 URL，每个 pathParam 唯一对应一段
  //
  // mode：
  //  - 'template' → 段本身是占位符（:id / {id}）。填值时 URL 保持占位符不变，
  //                 发送/cURL 时用 value 替换。key 可改名，同步回 URL。
  //  - 'literal'  → 段是字面量（123 / abc123 / UUID）。填值时改 URL 字面量；
  //                 key 为只读派生标签，不可改名。
  pathParams: Array<{ key: string; value: string; segment: string; mode: 'template' | 'literal' }>
  queryParams: Array<{ key: string; value: string }>  // [{key:"page", value:"1"}]
}

// ── HTTP 响应相关类型 ──────────────────────────────────────

export interface ParamItem {
  key: string
  value: string
  // 1.0.4 新增：字段类型（string/number/boolean/array/object/...，自由文本）。
  // 可选字段向后兼容旧数据（DB 中的旧 JSON 无此字段），渲染/写库时统一兜底为 ''
  type?: string
  // 1.0.4 新增：字段描述（可来自数据字典，见 descriptionDictRef）
  description?: string
  // 1.0.4 新增：引用的字典项 id（dictionary_items.id），无则 null
  descriptionDictRef?: number | null
  enabled: boolean
}

export interface SendRequestParams {
  method: string
  url: string
  query_params: ParamItem[]
  headers: ParamItem[]
  body_type: string
  body: string
  path_params: ParamItem[]
  auth_type?: string
  auth_config?: string
}

export interface HttpResponse {
  status_code: number
  status_text: string
  headers: [string, string][]
  body: string
  body_size: number
  elapsed_ms: number
  is_truncated: boolean
  history_id: number
}

export interface HistoryRecord {
  id: number
  request_id: number
  test_case_id: number | null   // 1.0.4：所属用例（null = 原始参数调试）
  status_code: number | null
  response_time_ms: number | null
  // 三个大字段：list_history 不返回（为 null），diff / 回填时用 get_history_record 补拉
  request_snapshot: string | null   // JSON
  response_body: string | null
  is_truncated: number       // 0 | 1
  response_headers: string | null   // JSON
  created_at: string
}

// ── 数据字典（1.0.4 新增）─────────────────────────────────────
export interface DataDictionary {
  id: number
  code: string
  name: string
  description: string
  builtin: number       // 0 | 1
  project_id: number | null
  created_at: string
  updated_at: string
}

export interface DictionaryItem {
  id: number
  dictionary_id: number
  label: string
  value: string
  description: string
  sort_order: number
}

// ── 字典「字段名绑定」（1.0.4）──────────────────────────────
export interface FieldDictionaryRule {
  id: number
  project_id: number
  field_name: string
  dictionary_id: number
}

export interface FieldDictionaryOverride {
  id: number
  project_id: number
  request_id: number
  field_name: string
  dictionary_id: number | null   // null = 解除绑定
}

// ── 压测历史（1.0.4 新增）─────────────────────────────────────
export interface StressRun {
  id: number
  request_id: number
  config_json: string
  stats_json: string
  created_at: string
}

// ── 环境相关类型 ──────────────────────────────────────

export interface Environment {
  id: number
  project_id: number
  name: string
  base_url: string | null
  is_active: number      // 0 | 1
  created_at: string
}

export interface EnvVariable {
  id: number
  env_id: number
  key: string
  value: string
  description: string | null
  enabled: number        // 0 | 1
}

// ── Cookie 类型 ──────────────────────────────────────

export interface CookieItem {
  id: number
  scope_type: string     // "global" | "project"
  project_id: number | null
  domain: string
  name: string
  value: string
  path: string
  expires_at: string | null
  http_only: number
  secure: number
  enabled: number
}

// ── 测试用例类型 ──────────────────────────────────────

/** 1.0.5：用例类型枚举（AI 技能分类 + 前端 badge/筛选用） */
export type CaseType =
  | 'happy_path'
  | 'missing_required'
  | 'unauthorized'
  | 'boundary'
  | 'empty_list'
  | 'type_error'
  | 'invalid_chars'

/** case_type → 中文标签（badge / 筛选器共用） */
export const CASE_TYPE_LABELS: Record<CaseType, string> = {
  happy_path: '正常',
  missing_required: '缺必填',
  unauthorized: '未授权',
  boundary: '边界值',
  empty_list: '空列表',
  type_error: '类型错误',
  invalid_chars: '特殊字符',
}

export interface TestCase {
  id: number
  request_id: number | null
  collection_id: number
  name: string
  description: string | null
  source: string            // "manual" | "ai_generated"
  case_type: CaseType       // 1.0.5
  method: string | null
  url: string | null
  headers: string           // JSON 数组
  params: string            // JSON 数组
  body_type: string | null
  body: string | null
  assertions: string        // JSON 数组
  last_run_at: string | null
  last_status: string       // "pending" | "passed" | "failed" | "error"
  last_duration_ms: number | null
  last_response: string | null
  starred: number           // 0 | 1
  enabled: number           // 0 | 1
  sort_order: number
  created_at: string
  updated_at: string
}

// ����ִ����ʷ��M3-C ���������� Rust types::TestCaseHistory ����
// ÿ������������ 10 ������ SQLite ������ trg_tch_keep_10 �Զ�������̭��
export interface TestCaseHistory {
  id: number
  test_case_id: number
  status_code: number | null    // null = �����ʧ�ܣ�DNS/��ʱ��
  duration_ms: number | null    // null = �����ʧ��
  response_preview: string | null  // ��ӦժҪ��ǰ�� ��1KB �ü���
  error_message: string | null     // ��������HTTP ����� status_code��
  created_at: string
}

// ── 断言（1.0.5 新增）────────────────────────────────────────

export type AssertionType = 'status_code' | 'json_path'
export type AssertionOperator = 'eq' | 'ne' | 'not_null' | 'contains'

/** 存在 test_cases.assertions 列里的单条断言 */
export interface Assertion {
  type: AssertionType
  /** json_path 专用，如 "$.code" / "$.data.list[0].id" */
  path?: string
  operator: AssertionOperator
  expected: string
}

/** run_test_case 返回的单条断言求值结果 */
export interface AssertionResult {
  kind: string
  path: string
  operator: string
  expected: string
  actual: string
  passed: boolean
  message: string
}

/** run_test_case 的返回值 */
export interface RunCaseResult {
  test_case_id: number
  status: 'passed' | 'failed' | 'error'
  status_code: number | null
  elapsed_ms: number
  response_body: string
  assertions: AssertionResult[]
  passed_count: number
  total_count: number
  error_message: string | null
}
