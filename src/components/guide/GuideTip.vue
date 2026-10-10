<template>
  <n-popover
    :show="visible"
    trigger="manual"
    placement="right"
    :show-arrow="true"
    style="max-width: 280px"
  >
    <template #trigger>
      <slot />
    </template>
    <div class="guide-tip">
      <div class="guide-tip__title">{{ content.title }}</div>
      <div class="guide-tip__body">{{ content.body }}</div>
      <n-button size="tiny" type="primary" @click="dismiss">知道了</n-button>
    </div>
  </n-popover>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NPopover, NButton } from 'naive-ui'
import { useGuideStore, type GuideId } from '../../stores/guide'
import { GUIDE_CONTENT } from '../../config/guideContent'

const props = defineProps<{ guideId: GuideId; active: boolean }>()
const guide = useGuideStore()
const content = GUIDE_CONTENT[props.guideId]

// CLAUDE.md gotcha：setup 顶层不解构 store 响应式值，用 computed 包
const visible = computed(() => props.active && !guide.isSeen(props.guideId))

function dismiss() {
  guide.markSeen(props.guideId)
}
</script>

<style scoped>
.guide-tip__title {
  font-weight: 600;
  margin-bottom: var(--spacing-xs);
  font-size: var(--font-size-base);
}
.guide-tip__body {
  margin-bottom: var(--spacing-sm);
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}
</style>
