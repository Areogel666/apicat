<template>
  <aside class="sidebar" :style="{ width: sidebarWidth + 'px' }">
    <!-- 搜索框 -->
    <div class="sidebar__search">
      <n-input
        v-model:value="searchText"
        placeholder="搜索接口..."
        size="small"
        clearable
      >
        <template #prefix>🔍</template>
      </n-input>
    </div>

    <!-- 接口树（M2 实现真实数据，M1 先展示占位） -->
    <div class="sidebar__tree">
      <n-empty
        v-if="!hasProject"
        description="暂无项目"
        size="small"
        style="margin-top: 40px"
      />
      <template v-else>
        <!-- M2 替换为真实 NTree -->
        <div class="sidebar__placeholder">
          <div class="tree-item">📁 用户模块</div>
          <div class="tree-item tree-item--child">📄 登录接口</div>
          <div class="tree-item tree-item--child">📄 注册接口</div>
          <div class="tree-item">📁 订单模块</div>
          <div class="tree-item tree-item--child">📄 创建订单</div>
        </div>
      </template>
    </div>

    <!-- 底部操作 -->
    <div class="sidebar__footer">
      <n-button size="small" block dashed>+ 新建接口</n-button>
    </div>
  </aside>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { NInput, NEmpty, NButton } from 'naive-ui'
import { useUiStore } from '../../stores/ui'

const uiStore = useUiStore()
const sidebarWidth = uiStore.sidebarWidth

const searchText = ref('')
// M2 起接入真实项目数据
const hasProject = ref(true)
</script>

<style scoped>
.sidebar {
  display: flex;
  flex-direction: column;
  height: 100%;
  border-right: 1px solid var(--n-border-color, #e0e0e6);
  background: var(--n-color, #fafafa);
  flex-shrink: 0;
  overflow: hidden;
}

.sidebar__search {
  padding: 10px 10px 6px;
  flex-shrink: 0;
}

.sidebar__tree {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
}

.sidebar__placeholder {
  padding: 4px 0;
}

.tree-item {
  padding: 6px 16px;
  font-size: 13px;
  cursor: pointer;
  border-radius: 4px;
  margin: 0 6px;
  transition: background 0.15s;
}

.tree-item:hover {
  background: var(--n-item-color-hover, rgba(0,0,0,0.06));
}

.tree-item--child {
  padding-left: 36px;
}

.sidebar__footer {
  padding: 8px 10px;
  border-top: 1px solid var(--n-border-color, #e0e0e6);
  flex-shrink: 0;
}
</style>
