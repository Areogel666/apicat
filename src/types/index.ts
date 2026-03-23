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
