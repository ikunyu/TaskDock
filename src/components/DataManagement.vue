<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from 'vue';
import { dataBridge } from '../services/data';
import { nativeMode } from '../services/bridge';
import type { BackupEntry, BackupSettings, DataStatus } from '../data-types';
import { formatInstant } from '../domain/tasks';
import { toAppError } from '../services/errors';
import AppButton from './AppButton.vue';
import ConfirmDialog from './ConfirmDialog.vue';
const emit = defineEmits<{ busy: [value: boolean] }>();
const status = ref<DataStatus | null>(null),
  busy = ref(false),
  error = ref(''),
  notice = ref(''),
  confirm = ref<BackupEntry | null>(null);
const draft = reactive<BackupSettings>({
  autoBackup: true,
  intervalHours: 24,
  retention: 30,
  version: 1,
});
let baseline = '',
  disposed = false,
  stop: (() => void) | undefined;
const dirty = computed(() => JSON.stringify(draft) !== baseline);
const stale = computed(() => !!status.value && draft.version !== status.value.settings.version);
async function refresh(reset = false) {
  try {
    const value = await dataBridge.status();
    if (disposed) return;
    const update = reset || !status.value || !dirty.value;
    status.value = value;
    if (update) {
      Object.assign(draft, value.settings);
      baseline = JSON.stringify(draft);
    }
  } catch (e) {
    error.value = toAppError(e).message;
  }
}
async function action(work: () => Promise<void>) {
  if (busy.value) return;
  busy.value = true;
  emit('busy', true);
  error.value = '';
  notice.value = '';
  try {
    await work();
  } catch (e) {
    error.value = toAppError(e).message;
  } finally {
    busy.value = false;
    emit('busy', false);
  }
}
function save() {
  void action(async () => {
    const value = await dataBridge.save({ ...draft });
    status.value = value;
    Object.assign(draft, value.settings);
    baseline = JSON.stringify(draft);
    notice.value = '备份设置已保存。';
  });
}
function backup() {
  void action(async () => {
    await dataBridge.backup();
    await refresh();
    notice.value = '一致性备份已创建，校验完成。';
  });
}
function exportFile(format: 'json' | 'csv') {
  void action(async () => {
    const file = await dataBridge.export(format);
    if (file) notice.value = `已导出：${file}`;
  });
}
function openDirectory() {
  void action(async () => {
    await dataBridge.openDirectory();
  });
}
function restore() {
  const entry = confirm.value;
  if (!entry) return;
  void action(async () => {
    const result = await dataBridge.restore(entry.id);
    confirm.value = null;
    await refresh(true);
    notice.value = `数据恢复完成，恢复前安全备份已保留。${result.warnings.join('；')}`;
  });
}
onMounted(async () => {
  stop = await dataBridge.onChange(() => {
    void refresh();
  });
  if (disposed) {
    stop();
    return;
  }
  await refresh();
});
onUnmounted(() => {
  disposed = true;
  stop?.();
  emit('busy', false);
});
</script>
<template>
  <section class="data-management" aria-label="数据管理">
    <h3>数据管理</h3>
    <p class="muted small">
      自动备份仅在应用运行时执行；手动及恢复前安全备份不会被自动保留策略清理。
    </p>
    <p v-if="error" role="alert" class="form-error">{{ error }}</p>
    <p v-if="status" class="muted small data-location">数据目录：{{ status.dataDirectory }}</p>
    <p v-if="status?.lastError" role="alert" class="form-error">{{ status.lastError }}</p>
    <p v-if="notice" role="status" class="success-note">{{ notice }}</p>
    <form @submit.prevent="save">
      <label class="setting-check"
        ><input
          v-model="draft.autoBackup"
          type="checkbox"
          :disabled="busy || !nativeMode"
        />自动备份</label
      >
      <div class="form-grid">
        <label
          >周期（小时）<input
            v-model.number="draft.intervalHours"
            type="number"
            min="1"
            max="8760"
            required
            :disabled="busy" /></label
        ><label
          >保留自动备份数量<input
            v-model.number="draft.retention"
            type="number"
            min="1"
            max="365"
            required
            :disabled="busy"
        /></label>
      </div>
      <p v-if="stale" class="note-card">备份配置已变更，输入已保留，请重新载入。</p>
      <div class="schedule-options">
        <AppButton type="submit" :disabled="busy || !status || stale || !nativeMode"
          >保存备份设置</AppButton
        ><AppButton :disabled="busy || !nativeMode" @click="refresh(true)">重新载入</AppButton>
      </div>
    </form>
    <p v-if="status?.lastSuccess || status?.lastAttempt" class="muted small">
      最近成功：{{ formatInstant(status?.lastSuccess ?? null) }} · 最近尝试：{{
        formatInstant(status?.lastAttempt ?? null)
      }}
    </p>
    <div class="schedule-options">
      <AppButton :disabled="busy || !nativeMode" @click="backup">立即备份</AppButton
      ><AppButton :disabled="busy || !nativeMode" @click="openDirectory">打开备份目录</AppButton
      ><AppButton :disabled="busy || !nativeMode" @click="exportFile('json')"
        >JSON完整导出</AppButton
      ><AppButton :disabled="busy || !nativeMode" @click="exportFile('csv')">CSV全部任务</AppButton>
    </div>
    <p class="muted small">CSV包含全部任务，JSON包含完整数据。当前不提供JSON导入。</p>
    <div class="backup-list">
      <article v-for="entry in status?.backups" :key="entry.id">
        <div>
          <strong>{{ formatInstant(entry.createdAt) }}</strong
          ><small
            >{{
              entry.kind === 'auto' ? '自动' : entry.kind === 'safety' ? '恢复前安全备份' : '手动'
            }}
            · Schema {{ entry.schemaVersion }} · {{ Math.ceil(entry.bytes / 1024) }} KB</small
          ><small class="backup-filename">{{ entry.filename }}</small>
        </div>
        <AppButton :disabled="busy" @click="confirm = entry">恢复</AppButton>
      </article>
    </div>
    <ConfirmDialog
      v-if="confirm"
      title="恢复数据库"
      :message="`将用 ${formatInstant(confirm.createdAt)} 的备份替换当前全部数据，所有窗口的未保存草稿将丢弃。恢复前会创建当前数据库安全备份；请确认其他窗口已完成编辑。`"
      confirm-label="确认替换当前数据"
      :busy="busy"
      @cancel="confirm = null"
      @confirm="restore"
    />
  </section>
</template>
