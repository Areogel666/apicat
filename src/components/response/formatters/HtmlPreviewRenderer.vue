<template>
  <div class="html-preview-wrapper">
    <iframe
      class="html-preview-frame"
      sandbox=""
      :srcdoc="body"
      referrerpolicy="no-referrer"
      title="HTML 响应预览"
    />
  </div>
</template>

<script setup lang="ts">
/**
 * HTML 预览渲染器（sandbox iframe）
 *
 * 安全策略：
 *   - sandbox=""（空字符串）= 最严格沙箱：
 *     禁脚本执行 / 禁 form / 禁 navigation / 禁同源 / 禁 popup
 *   - referrerpolicy="no-referrer"：iframe 内的图片/CSS 请求不带 Referer，
 *     避免泄露 ApiCat 的使用痕迹给远端服务器
 *   - 通过 :srcdoc 传入 body：字符串注入，不做 URL 跳转，无任何执行路径可以"逃逸"出沙箱
 *
 * 限制（本轮不处理）：
 *   - iframe 高度固定（sandbox 下无法 postMessage 做 auto-resize）
 *   - 内容内的相对链接可能 404（没 base href），可接受
 */

defineProps<{ body: string }>()
</script>

<style scoped>
.html-preview-wrapper {
  flex: 1;
  display: flex;
  overflow: hidden;
  border-radius: 4px;
  border: 1px solid var(--border-base);
  background: #fff;
}

.html-preview-frame {
  flex: 1;
  width: 100%;
  border: none;
  background: #fff;
}
</style>
