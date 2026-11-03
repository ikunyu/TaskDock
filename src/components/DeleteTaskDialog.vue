<script setup lang="ts">
import type { Task } from '../task-types';
import ConfirmDialog from './ConfirmDialog.vue';

defineProps<{ task: Task; busy?: boolean; error?: string; unsaved?: boolean }>();
const emit = defineEmits<{ confirm: []; cancel: [] }>();
</script>

<template>
  <ConfirmDialog
    title="永久删除任务"
    :message="`确定永久删除“${task.title}”吗？任务及其子任务、提醒将被删除，无法撤销或恢复。${unsaved ? '未保存的修改也会丢弃。' : ''}${task.schedule?.seriesId ? '只删除这一次，不会停止后续重复任务。' : ''}`"
    confirm-label="永久删除"
    danger
    :busy="busy"
    :error="error"
    @cancel="emit('cancel')"
    @confirm="emit('confirm')"
  />
</template>
