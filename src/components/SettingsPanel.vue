<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref, watch } from 'vue';
import { useAppStore } from '../stores/app';
import { bridge, nativeMode } from '../services/bridge';
import { toAppError } from '../services/errors';
import DialogFrame from './DialogFrame.vue';
import AppButton from './AppButton.vue';
import ThemeSelect from './ThemeSelect.vue';
import { widgetFontOptions } from '../desktop-types';
import DataManagement from './DataManagement.vue';
import UpdatePanel from './UpdatePanel.vue';
const dataBusy = ref(false);
const tab = ref<'desktop' | 'data' | 'status'>('desktop');
const emit = defineEmits<{ close: [] }>();
const app = useAppStore();
const draft = reactive({ ...app.desktop });
const baseRevision = ref(app.revision);
const error = ref(''),
  notice = ref('');
watch(
  draft,
  () => {
    notice.value = '';
  },
  { flush: 'sync' },
);
const stale = computed(() => baseRevision.value !== app.revision);
let timer: ReturnType<typeof setInterval> | undefined;
function reload() {
  Object.assign(draft, app.desktop);
  baseRevision.value = app.revision;
  error.value = '';
  notice.value = '';
}
function resetFonts() {
  for (const font of widgetFontOptions) draft[font.key] = font.defaultValue;
}
async function save() {
  error.value = '';
  notice.value = '';
  if (
    !Number.isInteger(draft.widgetOpacity) ||
    draft.widgetOpacity < 20 ||
    draft.widgetOpacity > 100
  ) {
    error.value = '不透明度需为 20–100。';
    return;
  }
  for (const font of widgetFontOptions) {
    const value = draft[font.key];
    if (!Number.isInteger(value) || value < font.min || value > font.max) {
      error.value = `${font.label}字号需为 ${font.min}–${font.max}。`;
      return;
    }
  }
  try {
    await app.saveDesktop({ ...draft }, baseRevision.value);
    Object.assign(draft, app.desktop);
    baseRevision.value = app.revision;
    notice.value = '桌面设置已保存并应用。';
  } catch (value) {
    error.value = toAppError(value).message;
  }
}
async function reset() {
  error.value = '';
  try {
    await bridge.resetPositions();
    notice.value = '窗口已恢复到当前主显示器的安全默认位置。';
  } catch (value) {
    error.value = toAppError(value).message;
  }
}
async function exit() {
  try {
    await bridge.exitApp();
  } catch (value) {
    error.value = toAppError(value).message;
  }
}
onMounted(() => {
  void app.refreshDesktopStatus();
  timer = setInterval(() => {
    void app.refreshDesktopStatus();
  }, 2000);
});
onUnmounted(() => {
  if (timer) clearInterval(timer);
});
</script>
<template>
  <DialogFrame title="软件设置" :busy="app.busy || dataBusy" @close="emit('close')">
    <div class="desktop-settings">
      <div class="settings-tabs" role="tablist" aria-label="设置分组">
        <button
          v-for="item in [
            { id: 'desktop', label: '外观与桌面' },
            { id: 'data', label: '数据管理' },
            { id: 'status', label: '关于与运行状态' },
          ] as const"
          :id="`settings-tab-${item.id}`"
          :key="item.id"
          type="button"
          role="tab"
          :aria-selected="tab === item.id"
          :aria-controls="`settings-page-${item.id}`"
          :disabled="app.busy || dataBusy"
          @click="tab = item.id"
        >
          {{ item.label }}
        </button>
      </div>
      <section
        v-show="tab === 'desktop'"
        id="settings-page-desktop"
        role="tabpanel"
        aria-labelledby="settings-tab-desktop"
        class="settings-page"
      >
        <h3>外观</h3>
        <ThemeSelect id="settings-appearance-theme" :disabled="dataBusy" />
        <p class="muted small">外观立即保存；以下桌面设置点击保存后生效。</p>
        <form @submit.prevent="save">
          <h3>悬浮小组件</h3>
          <div v-if="stale" class="note-card">
            设置已在其他操作中更新，当前输入保留。<AppButton @click="reload"
              >重新载入设置</AppButton
            >
          </div>
          <label
            >悬浮窗不透明度 {{ draft.widgetOpacity }}%<input
              v-model.number="draft.widgetOpacity"
              type="range"
              min="20"
              max="100"
              step="1"
              :disabled="app.busy || dataBusy"
          /></label>
          <label v-for="font in widgetFontOptions" :key="font.key">
            {{ font.label }}字号 {{ draft[font.key] }} px
            <input
              v-model.number="draft[font.key]"
              type="range"
              :min="font.min"
              :max="font.max"
              step="1"
              :disabled="app.busy || dataBusy"
            />
          </label>
          <div>
            <AppButton :disabled="app.busy || dataBusy" @click="resetFonts">默认字号</AppButton>
          </div>
          <label class="setting-check"
            ><input
              v-model="draft.widgetTopmost"
              type="checkbox"
              :disabled="app.busy || dataBusy"
            />悬浮窗置顶</label
          >
          <label class="setting-check"
            ><input
              v-model="draft.widgetLocked"
              type="checkbox"
              :disabled="app.busy || dataBusy"
            />锁定悬浮窗（鼠标穿透）</label
          >
          <p class="muted small">锁定后可右键系统托盘，选择“解锁悬浮窗”。</p>
          <label class="setting-check"
            ><input
              v-model="draft.widgetVisible"
              type="checkbox"
              :disabled="app.busy || dataBusy"
            />显示悬浮窗（重启后保留）</label
          >
          <h3>快捷键与启动</h3>
          <label
            >全局快捷键<input
              v-model="draft.shortcut"
              maxlength="100"
              :disabled="app.busy || dataBusy"
              placeholder="Ctrl+Alt+T"
          /></label>
          <p class="muted small">
            支持 Ctrl、Alt、Shift 加字母、数字或 F1–F12；必须包含 Ctrl 或 Alt。
          </p>
          <label class="setting-check"
            ><input
              v-model="draft.autostart"
              type="checkbox"
              :disabled="app.busy || dataBusy"
            />开机自动启动</label
          >
          <p class="muted small">
            仅在勾选并保存后注册。登录时在托盘后台运行，按保存的设置显示悬浮窗。
          </p>
          <label
            >关闭主窗口时<select v-model="draft.closeBehavior" :disabled="app.busy || dataBusy">
              <option value="hide">隐藏到系统托盘</option>
              <option value="exit">退出 TaskDock</option>
            </select></label
          >
          <p v-if="error" class="form-error" role="alert">{{ error }}</p>
          <p v-if="notice" class="success-note" role="status">{{ notice }}</p>
          <AppButton
            type="submit"
            variant="primary"
            :disabled="app.busy || dataBusy || stale || !nativeMode || !app.desktopStatus.ready"
            >{{ app.busy || dataBusy ? '正在应用…' : '保存桌面设置' }}</AppButton
          >
        </form>
      </section>
      <section
        v-show="tab === 'status'"
        id="settings-page-status"
        role="tabpanel"
        aria-labelledby="settings-tab-status"
        class="settings-page"
      >
        <h3>TaskDock {{ app.appVersion }}</h3>
        <UpdatePanel :disabled="dataBusy || app.busy" />
        <p class="muted small">本地工作台 · Schema {{ app.schemaVersion }} · 未签名构建</p>
        <div class="desktop-runtime muted small">
          <p>
            当前有效快捷键：{{
              app.desktopStatus.effectiveShortcut ?? '未注册，可使用托盘快速添加'
            }}
          </p>
          <p>
            系统启动项：{{
              app.desktopStatus.autostartEnabled === null
                ? '无法读取'
                : app.desktopStatus.autostartEnabled
                  ? '已启用'
                  : '已关闭'
            }}
          </p>
          <p v-for="warning in app.desktopStatus.warnings" :key="warning" role="alert">
            {{ warning }}
          </p>
        </div>
      </section>
      <section
        v-show="tab === 'data'"
        id="settings-page-data"
        role="tabpanel"
        aria-labelledby="settings-tab-data"
        class="settings-page"
      >
        <DataManagement @busy="dataBusy = $event" />
      </section>
      <div class="dialog-actions">
        <AppButton
          v-if="tab === 'desktop'"
          :disabled="app.busy || dataBusy || !nativeMode"
          @click="reset"
          >恢复窗口默认位置</AppButton
        ><AppButton :disabled="app.busy || dataBusy || !nativeMode" @click="exit"
          >退出 TaskDock</AppButton
        >
      </div>
    </div>
  </DialogFrame>
</template>
