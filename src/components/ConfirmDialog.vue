<script setup lang="ts">
import DialogFrame from './DialogFrame.vue';
import AppButton from './AppButton.vue';
withDefaults(
  defineProps<{
    title?: string;
    message: string;
    confirmLabel?: string;
    busy?: boolean;
    danger?: boolean;
    error?: string;
  }>(),
  { title: '请确认', confirmLabel: '确认' },
);
const emit = defineEmits<{ confirm: []; cancel: [] }>();
</script>
<template>
  <DialogFrame :title="title" :busy="busy" @close="emit('cancel')"
    ><p class="dialog-message">{{ message }}</p>
    <p v-if="error" class="form-error" role="alert">{{ error }}</p>
    <div class="dialog-actions">
      <AppButton :disabled="busy" @click="emit('cancel')">取消</AppButton
      ><AppButton
        :variant="danger ? 'danger' : 'primary'"
        :disabled="busy"
        @click="emit('confirm')"
        >{{ busy ? '正在处理…' : confirmLabel }}</AppButton
      >
    </div></DialogFrame
  >
</template>
