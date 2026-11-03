<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref } from 'vue';
import AppButton from '../components/AppButton.vue';
import AppIcon from '../components/AppIcon.vue';
import DateField from '../components/DateField.vue';
import { bridge } from '../services/bridge';
import { useAppStore } from '../stores/app';
import { useTasksStore } from '../stores/tasks';
import { blankDraft, validDate, validateCreationDates } from '../domain/tasks';
import { toAppError } from '../services/errors';
const title = ref(''),
  category = ref<string | null>(null),
  error = ref(''),
  success = ref('');
const input = ref<HTMLInputElement>();
const app = useAppStore(),
  store = useTasksStore();
const plannedDate = ref(store.today);
let defaultDay = store.today;
async function hide() {
  if (store.busy) return;
  try {
    await bridge.hideCurrent();
  } catch (value) {
    app.fail(value);
  }
}
function focusInput() {
  if (!title.value.trim() && plannedDate.value === defaultDay) {
    plannedDate.value = store.today;
    defaultDay = store.today;
  }
  input.value?.focus();
}
function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    event.preventDefault();
    void hide();
  }
}
async function save() {
  store.tick();
  error.value = '';
  success.value = '';
  if (!title.value.trim() || [...title.value.trim()].length > 500) {
    error.value = '标题不能为空，且最多 500 字。';
    return;
  }
  if (!validDate(plannedDate.value)) {
    error.value = '请选择任务日期。';
    return;
  }
  try {
    const draft = blankDraft(plannedDate.value);
    draft.title = title.value;
    draft.dueDate = plannedDate.value;
    draft.categoryId = category.value;
    const invalid = validateCreationDates(draft);
    if (invalid) {
      error.value = invalid;
      return;
    }
    await store.create(draft);
    title.value = '';
    success.value = plannedDate.value <= store.today ? '已添加到今天。' : '已添加到未来事项。';
    await nextTick();
    focusInput();
  } catch (value) {
    error.value = toAppError(value).message;
  }
}
let stopQuick: (() => void) | undefined;
let quickDisposed = false;
onMounted(() => {
  void bridge
    .onQuickFocus(async () => {
      await nextTick();
      focusInput();
    })
    .then((stop) => {
      if (quickDisposed) stop();
      else stopQuick = stop;
    });
  window.addEventListener('focus', focusInput);
  window.addEventListener('keydown', onKeydown);
  focusInput();
});
onUnmounted(() => {
  quickDisposed = true;
  stopQuick?.();
  window.removeEventListener('focus', focusInput);
  window.removeEventListener('keydown', onKeydown);
});
</script>
<template>
  <form class="quick-layout" @submit.prevent="save">
    <header class="section-heading">
      <span class="mini-brand"><span class="brand-mark tiny">✓</span>快速输入</span
      ><span class="phase-badge">安排一件要做的事</span>
    </header>
    <label for="quick-title" class="quick-label">有什么需要完成？</label
    ><input
      id="quick-title"
      ref="input"
      v-model="title"
      autofocus
      class="quick-input"
      maxlength="500"
      :disabled="store.busy"
      placeholder="输入一个任务，按 Enter 保存…"
      autocomplete="off"
    />
    <div class="quick-meta quick-options">
      <DateField
        :model-value="plannedDate"
        @update:model-value="plannedDate = $event ?? ''"
        label="截止日期"
        :min="store.today"
        required
        :disabled="store.busy"
      />
      <label
        >分类<select v-model="category" :disabled="store.busy">
          <option :value="null">无分类</option>
          <option v-for="item in store.categories" :key="item.id" :value="item.id">
            {{ item.name }}
          </option>
        </select></label
      >
    </div>
    <p v-if="error" class="form-error" role="alert">{{ error }}</p>
    <p v-else-if="success" class="success-note" role="status">{{ success }}</p>
    <footer class="quick-actions">
      <span class="muted small">Enter 保存 · Esc 隐藏</span
      ><AppButton type="submit" :disabled="store.busy || !store.ready" variant="primary"
        ><AppIcon name="plus" :size="16" />{{ store.busy ? '正在保存…' : '添加任务' }}</AppButton
      >
    </footer>
  </form>
</template>
