<template>
  <div class="add-cookie-form">
    <div v-if="!adding" style="margin-top: 4px">
      <n-button size="small" dashed style="width:100%" @click="adding = true">
        + 添加 Cookie
      </n-button>
    </div>
    <div v-else class="form-row">
      <n-input v-model:value="form.domain" size="small" placeholder="域名 如 api.example.com" style="flex:1.2" />
      <n-input v-model:value="form.name" size="small" placeholder="名称" style="flex:1" />
      <n-input v-model:value="form.value" size="small" placeholder="值" style="flex:2" />
      <n-input v-model:value="form.path" size="small" placeholder="路径 /" style="width:60px; flex-shrink:0" />
      <n-button size="small" type="primary" :disabled="!form.domain || !form.name" @click="handleAdd">确定</n-button>
      <n-button size="small" @click="adding = false">取消</n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { NButton, NInput } from 'naive-ui'
import { useCookieStore } from '../../stores/cookie'

const props = defineProps<{
  scopeType: string
  projectId: number | null
}>()
const emit = defineEmits<{ created: [] }>()

const cookieStore = useCookieStore()
const adding = ref(false)
const form = reactive({ domain: '', name: '', value: '', path: '/' })

async function handleAdd() {
  if (!form.domain || !form.name) return
  await cookieStore.createCookie(props.scopeType, props.projectId, form.domain, form.name, form.value, form.path)
  Object.assign(form, { domain: '', name: '', value: '', path: '/' })
  adding.value = false
  emit('created')
}
</script>

<style scoped>
.add-cookie-form { margin-top: 8px; }
.form-row { display: flex; gap: 4px; align-items: center; }
</style>
