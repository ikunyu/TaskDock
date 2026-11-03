<script setup lang="ts">
import { computed, ref } from 'vue';
import { useTasksStore } from '../stores/tasks';
import { formatInstant } from '../domain/tasks';
import AppButton from './AppButton.vue';
const emit = defineEmits<{ open: [id: string] }>();
const store = useTasksStore();
const error = ref('');
const limit = ref(20);
const due = computed(() => store.reminders.filter((r) => r.state === 'notified'));
async function act(id: string, minutes: number | null) {
  error.value = '';
  try {
    await store.reminderAction(id, minutes);
  } catch (e) {
    error.value = (e as { message?: string }).message ?? '提醒操作失败，请重试。';
  }
}
</script>
<template>
  <section v-if="due.length" class="reminder-center" aria-label="提醒中心" aria-live="polite">
    <div class="section-heading">
      <strong>提醒中心 · {{ due.length }}</strong
      ><span class="muted small">应用内记录 · 不依赖系统通知权限</span>
    </div>
    <p v-if="error" class="form-error" role="alert">{{ error }}</p>
    <article v-for="r in due.slice(0, limit)" :key="r.id" class="reminder-item">
      <button class="task-title-button" @click="emit('open', r.taskId)">{{ r.title }}</button>
      <small class="muted">{{ formatInstant(r.scheduledAt) }} · 到期提醒</small>
      <div class="schedule-options">
        <AppButton
          v-for="[m, text] in [
            [10, '10分钟'],
            [30, '30分钟'],
            [60, '1小时'],
          ] as const"
          :key="m"
          :disabled="store.busy"
          @click="act(r.id, m)"
          >稍后{{ text }}</AppButton
        ><AppButton :disabled="store.busy" @click="act(r.id, null)">已知晓</AppButton>
      </div>
    </article>
    <AppButton v-if="due.length > limit" @click="limit += 20">显示更多提醒</AppButton>
  </section>
</template>
