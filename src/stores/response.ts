import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { HttpResponse, SendRequestParams } from '../types'

export const useResponseStore = defineStore('response', () => {
  const response = ref<HttpResponse | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function sendRequest(
    requestId: number,
    params: SendRequestParams,
    envId: number | null,
    projectId: number | null,
  ): Promise<HttpResponse | null> {
    loading.value = true
    error.value = null
    try {
      response.value = await invoke<HttpResponse>('send_request', {
        request_id: requestId,
        params,
        env_id: envId,
        project_id: projectId,
      })
    } catch (e) {
      error.value = String(e)
      response.value = null
    } finally {
      loading.value = false
    }
    return response.value
  }

  function clear() {
    response.value = null
    error.value = null
  }

  return { response, loading, error, sendRequest, clear }
})
