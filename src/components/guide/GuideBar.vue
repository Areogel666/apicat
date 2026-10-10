<template>
  <n-alert v-if="!guide.isSeen(guideId)" type="info" :show-icon="true" class="guide-bar" @close="dismiss">
    <b>{{ content.title }}：</b>{{ content.body }}
  </n-alert>
</template>

<script setup lang="ts">
import { NAlert } from 'naive-ui'
import { useGuideStore, type GuideId } from '../../stores/guide'
import { GUIDE_CONTENT } from '../../config/guideContent'

const props = defineProps<{ guideId: GuideId }>()
const guide = useGuideStore()
const content = GUIDE_CONTENT[props.guideId]

function dismiss() {
  guide.markSeen(props.guideId)
}
</script>

<style scoped>
.guide-bar {
  margin-bottom: var(--spacing-sm);
}
</style>
