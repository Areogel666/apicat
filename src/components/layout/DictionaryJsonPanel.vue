<template>
  <div class="dict-panel">
    <!-- 顶栏双 tab（1.0.5 改版）-->
    <n-tabs v-model:value="activeTab" type="line" size="small" class="dict-tabs">
      <n-tab name="preview">字典预览</n-tab>
      <n-tab name="rules">
        字典绑定规则
        <n-tag v-if="bindingCount" size="tiny" :bordered="false" style="margin-left: 4px">
          {{ bindingCount }}
        </n-tag>
      </n-tab>
    </n-tabs>

    <!-- 两块面板 v-show 常驻（项目规范：切换保留浏览状态，不卸载重挂）-->
    <DictPreviewTab v-show="activeTab === 'preview'" @goto-rules="gotoRules" />
    <DictRulesTab
      v-show="activeTab === 'rules'"
      v-model:filter-dict-id="rulesFilterDictId"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { NTabs, NTab, NTag } from 'naive-ui'
import { useDictionaryStore } from '../../stores/dictionary'
import DictPreviewTab from '../dictionary/DictPreviewTab.vue'
import DictRulesTab from '../dictionary/DictRulesTab.vue'

const store = useDictionaryStore()

const activeTab = ref<'preview' | 'rules'>('preview')
// 从字典预览页「查看绑定规则」跳转过来时的字典筛选
const rulesFilterDictId = ref<number | null>(null)

const bindingCount = computed(() => {
  const { rules, overrides } = store.allFieldBindings()
  return rules.length + overrides.length
})

function gotoRules(dictId: number) {
  rulesFilterDictId.value = dictId
  activeTab.value = 'rules'
}
</script>

<style scoped>
.dict-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  padding: var(--spacing-sm) var(--spacing-lg) var(--spacing-md);
  background: var(--bg-elevated);
}

.dict-tabs {
  flex-shrink: 0;
  margin-bottom: var(--spacing-sm);
}
</style>
