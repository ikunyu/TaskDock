<script setup lang="ts">
import { computed, ref } from 'vue';
import { useUpdatesStore } from '../stores/updates';
import { nativeMode } from '../services/bridge';
import AppButton from './AppButton.vue';
import ConfirmDialog from './ConfirmDialog.vue';
const props = defineProps<{ compact?: boolean; disabled?: boolean }>();
const updates = useUpdatesStore();
const confirm = ref(false);
const visible = computed(
  () =>
    !props.compact ||
    ['downloading', 'ready'].includes(updates.state.phase) ||
    (updates.state.phase === 'error' && !!updates.state.availableBuild) ||
    (updates.state.phase === 'available' &&
      updates.dismissedBuild !== updates.state.availableBuild),
);
async function install() {
  confirm.value = false;
  await updates.act('install_update');
}
</script>
<template>
  <section
    v-if="visible"
    :class="compact ? 'update-notice' : 'update-panel'"
    aria-label="软件更新"
    aria-live="polite"
  >
    <strong>{{
      updates.state.phase === 'ready'
        ? '更新已下载，签名验证完成'
        : updates.state.phase === 'downloading'
          ? '正在下载更新'
          : updates.state.phase === 'error' && compact
            ? '更新未完成'
            : compact
              ? '有可用更新'
              : '软件更新'
    }}</strong>
    <p v-if="!compact && updates.state.currentBuild" class="muted small">
      当前构建 {{ updates.state.currentBuild }}
    </p>
    <p v-if="!compact && updates.state.notes" class="update-notes">{{ updates.state.notes }}</p>
    <div class="schedule-options">
      <AppButton
        v-if="!compact"
        :disabled="disabled || updates.busy || !nativeMode || updates.state.phase === 'ready'"
        @click="updates.act('check_for_updates')"
        >{{ updates.state.phase === 'checking' ? '正在检查…' : '检查更新' }}</AppButton
      >
      <AppButton
        v-if="
          updates.state.phase === 'available' ||
          (updates.state.phase === 'error' && updates.state.availableBuild)
        "
        variant="primary"
        :disabled="disabled || updates.busy"
        @click="updates.act('download_update')"
        >下载更新</AppButton
      >
      <span v-if="updates.state.phase === 'downloading'" class="muted small">{{
        updates.percent === null ? '下载中…' : `${updates.percent}%`
      }}</span>
      <AppButton
        v-if="updates.state.phase === 'ready'"
        variant="primary"
        :disabled="disabled || updates.busy"
        @click="confirm = true"
        >安装并重启</AppButton
      >
      <AppButton
        v-if="compact && updates.state.phase === 'available'"
        variant="ghost"
        @click="updates.dismissedBuild = updates.state.availableBuild"
        >稍后</AppButton
      >
    </div>
    <p v-if="!compact && updates.state.phase === 'up_to_date'" class="muted small">
      当前已是最新构建。
    </p>
    <p
      v-if="(updates.error || updates.state.message) && (!compact || updates.state.availableBuild)"
      class="form-error"
      role="alert"
    >
      {{ updates.error || updates.state.message }}
    </p>
  </section>
  <ConfirmDialog
    v-if="confirm"
    title="安装更新并重启"
    message="请先保存主窗口、悬浮窗和快速输入窗口中的编辑内容。安装会关闭所有窗口并重新启动，已保存的任务和备份保留在当前目录。"
    confirm-label="安装并重启"
    @cancel="confirm = false"
    @confirm="install"
  />
</template>
