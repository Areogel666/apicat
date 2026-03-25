// 与 Rust types.rs 保持一致的前端类型定义

export interface Project {
  id: number
  name: string
  description: string | null
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
  updated_at: string
}

// ── 压测类型 ──────────────────────────────────────────────
export interface StressStats {
  total: number
  success: number
  failed: number
  success_rate: number   // 0.0 ~ 100.0
  avg_ms: number
  p50_ms: number
  p95_ms: number
  p99_ms: number
  tps: number
  elapsed_sec: number
  done: boolean
}

export interface StressConfig {
  concurrent: number     // 1 ~ 500
  mode: 'count' | 'duration'
  value: number          // 总请求数 或 持续秒数
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
  sort_order: number
  created_at: string
  updated_at: string
}

// NTree 节点类型
export interface TreeNode {
  key: string            // 格式: "col-{id}" 或 "req-{id}"
  label: string
  type: 'collection' | 'request'
  data: Collection | ApiRequest
  children?: TreeNode[]
  isLeaf?: boolean
  prefix?: string        // emoji 图标
}

// URL 解析结果
export interface ParsedUrl {
  displayName: string       // "POST /apm/intl/download"
  pathTemplate: string      // "/apm/intl/download/{id}"
  pathParams: Array<{ key: string; value: string }>   // [{key:"{id}", value:"1676657"}]
  queryParams: Array<{ key: string; value: string }>  // [{key:"page", value:"1"}]
}

// ── HTTP 响应相关类型 ──────────────────────────────────────

export interface ParamItem {
  key: string
  value: string
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
  status_code: number | null
  response_time_ms: number | null
  request_snapshot: string   // JSON
  response_body: string
  is_truncated: number       // 0 | 1
  response_headers: string   // JSON
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

export interface TestCase {
  id: number
  request_id: number | null
  collection_id: number
  name: string
  description: string | null
  source: string            // "manual" | "ai_generated"
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
