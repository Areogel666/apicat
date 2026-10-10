/** 新手指引文案集中地。改文案只改这里；新增引导先加 GuideId 再补这里。 */
import type { GuideId } from '../stores/guide'

export interface GuideContent {
  title: string
  body: string
}

export const GUIDE_CONTENT: Record<GuideId, GuideContent> = {
  dictionary: {
    title: '数据字典',
    body: '把接口文档里反复出现的字段（如 code/msg/data）绑定为统一字典，描述与枚举自动带入参数表。点左侧 📖 按钮管理你的字典。',
  },
  stress: {
    title: '压测',
    body: '对任意接口发起并发压测，实时查看 QPS、P95/P99 延迟与错误率，报告可导出 Markdown。',
  },
  cookie: {
    title: 'Cookie 管理',
    body: '手动维护 Cookie 库，支持 全局/项目/环境 三级作用域，发送请求时按 URL 自动匹配注入；响应的 Set-Cookie 也可自动回存。',
  },
  skills: {
    title: 'AI 技能',
    body: '安装后可在 Claude Code 等 Agent 里直接读写 ApiCat 的接口、用例与字典（经本机 HTTP Bridge，数据不出本机）。在 Agent 会话里直接说"帮我加个接口"即可使用。',
  },
  import: {
    title: '导入接口',
    body: '支持 Postman Collection v2.1、OpenAPI 3.x（JSON/YAML）一键导入——把现有 Postman 工作区搬过来，接口树即刻就绪。',
  },
  env: {
    title: '环境变量',
    body: '为不同环境（开发/测试/生产）维护变量集，URL 与参数里写 {{base_url}} 即可切换环境一键生效。',
  },
  stats: {
    title: '使用统计',
    body: '这里汇总你的调试工作量：请求趋势、成功率、方法分布、字典与文档建设情况。全部数据仅保存在本机。',
  },
  firstRequest: {
    title: '发出第一个请求',
    body: '左侧已备好「示例：」接口，选中后直接点发送即可看到真实响应；也可以从顶部 ⚙️ 菜单导入你的 Postman Collection。',
  },
}
