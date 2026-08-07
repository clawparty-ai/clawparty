<template>
  <!-- 放在聊天页顶部。断线时显示，重连成功自动消失 -->
  <Transition name="slide">
    <div v-if="banner.visible" class="ws-banner" :class="banner.type">
      <span v-if="banner.type === 'reconnecting'">
        🟡 连接已断开，正在重连（第 {{ banner.attempt }} 次，{{ banner.nextRetrySec }}s 后重试）…
      </span>
      <span v-else-if="banner.type === 'restored'">🟢 连接已恢复，消息已同步</span>
    </div>
  </Transition>
</template>

<script setup>
import { reactive, watch } from 'vue';

const props = defineProps({
  // 由父组件把 ZAgentWS 实例传进来
  wsClient: { type: Object, required: true },
});

const banner = reactive({
  visible: false,
  type: 'reconnecting', // 'reconnecting' | 'restored'
  attempt: 0,
  nextRetrySec: 0,
});

let restoreTimer = null;

// 接管状态回调（如果父组件已设置 onStateChange，请在父组件里链式调用这里）
props.wsClient.onStateChange = (state, info) => {
  if (state === 'reconnecting') {
    banner.visible = true;
    banner.type = 'reconnecting';
    banner.attempt = info.attempt;
    banner.nextRetrySec = Math.max(1, Math.round((info.nextRetryMs || 0) / 1000));
    clearTimeout(restoreTimer);
  } else if (state === 'connected') {
    if (banner.visible && banner.type === 'reconnecting') {
      banner.type = 'restored';
      restoreTimer = setTimeout(() => { banner.visible = false; }, 3000);
    }
  }
};

// 发送按钮联动：断线期间拦截发送，避免消息静默进列表
const emit = defineEmits(['can-send-change']);
watch(() => banner.visible && banner.type === 'reconnecting', (blocked) => {
  emit('can-send-change', !blocked);
}, { immediate: true });
</script>

<style scoped>
.ws-banner {
  position: sticky;
  top: 0;
  z-index: 50;
  padding: 8px 16px;
  text-align: center;
  font-size: 13px;
}
.ws-banner.reconnecting { background: #fef3c7; color: #92400e; }
.ws-banner.restored { background: #d1fae5; color: #065f46; }
.slide-enter-active, .slide-leave-active { transition: all 0.3s ease; }
.slide-enter-from, .slide-leave-to { opacity: 0; transform: translateY(-100%); }
</style>
