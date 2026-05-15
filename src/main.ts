import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
// M3-B 主题底座：CSS 变量 token 体系，所有自定义组件引用
import './styles/tokens.css'

const app = createApp(App)
app.use(createPinia())
app.mount('#app')
