<template>
  <n-alert v-if="show" type="info" :show-icon="true" closable class="guide-bar" @close="show = false">
    <b>{{ content.title }}：</b>{{ content.body }}
  </n-alert>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { NAlert } from 'naive-ui'
import { useGuideStore, type GuideId } from '../../stores/guide'
import { GUIDE_CONTENT } from '../../config/guideContent'

const props = defineProps<{ guideId: GuideId }>()
const guide = useGuideStore()
const content = GUIDE_CONTENT[props.guideId]

// 一次性语义：提示条只在首次打开所在弹窗时出现，「显示」即记 seen，之后不再弹。
// 本地 show 与 seen 解耦：mount 时已记 seen，但本条仍展示到被关闭/随弹窗卸载为止。
// 弹窗关闭即卸载（naive 默认 display-directive=if），下次打开按 seen 不再显示。
const show = ref(!guide.isSeen(props.guideId))
onMounted(() => guide.markSeen(props.guideId))
</script>

<style scoped>
.guide-bar {
  margin-bottom: var(--spacing-sm);
}
</style>
