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
  // tour 的分步文案在下方 TOUR_STEPS；此处条目仅为满足 Record 全覆盖，不被 GuideTip/GuideBar 消费
  tour: { title: '功能导览', body: '' },
}

/** 首启 tour（driver.js 聚光灯）分步文案。panel = 进入该步前切换的左侧面板；null = 不切换 */
export interface TourStep {
  panel: 'interface' | 'dictionary' | 'docs' | 'stats' | null
  element: string
  title: string
  body: string
}

export const TOUR_STEPS: readonly TourStep[] = [
  {
    panel: 'interface', element: '.sidebar', title: '🔗 接口树',
    body: '所有接口与目录都在这里管理，选中接口即可发起调试。点文件夹名可折叠/展开，支持搜索与拖拽排序。',
  },
  {
    panel: 'dictionary', element: '.dict-sidebar', title: '📖 数据字典',
    body: '把 code/msg/data 这类反复出现的字段定义成字典并绑定字段名，参数的描述与枚举会自动带入，一处修改处处生效。',
  },
  {
    panel: 'docs', element: '.docs-sidebar', title: '📄 接口文档',
    body: '根据接口定义与用例一键生成 Markdown 文档，也可以直接浏览项目的文档输出目录。',
  },
  {
    panel: 'stats', element: '.stats-panel', title: '📊 使用统计',
    body: '请求趋势、成功率、方法分布一目了然，支持按项目/接口级联筛选。全部数据仅保存在本机。',
  },
  {
    panel: null, element: '#topbar-more-btn', title: '⚙️ 更多能力',
    body: '环境变量、AI 技能、导入导出、压测、主题等入口都收在这个菜单里——第一次上手建议从「导入接口」开始。',
  },
]
