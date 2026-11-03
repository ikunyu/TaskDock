<script setup lang="ts">
import { ref } from 'vue';
import type { Category } from '../task-types';
import { useTasksStore } from '../stores/tasks';
import { toAppError } from '../services/errors';
import DialogFrame from './DialogFrame.vue';
import ConfirmDialog from './ConfirmDialog.vue';
import AppButton from './AppButton.vue';
const emit = defineEmits<{ close: [] }>();
const store = useTasksStore();
const selected = ref<Category | null>(null),
  removing = ref<Category | null>(null);
const name = ref(''),
  color = ref('#6558d4'),
  error = ref('');
function edit(category: Category | null) {
  selected.value = category ? { ...category } : null;
  name.value = category?.name ?? '';
  color.value = category?.color ?? '#6558d4';
  error.value = '';
}
async function save() {
  if (!name.value.trim() || [...name.value.trim()].length > 100) {
    error.value = '分类名称不能为空，且最多 100 字。';
    return;
  }
  try {
    await store.saveCategory({
      id: selected.value?.id ?? null,
      expectedVersion: selected.value?.version ?? null,
      name: name.value,
      color: color.value,
      sortOrder: selected.value?.sortOrder ?? store.categories.length * 1024,
    });
    edit(null);
  } catch (value) {
    error.value = toAppError(value).message;
  }
}
async function remove(category: Category) {
  try {
    await store.deleteCategory(category);
    removing.value = null;
    if (selected.value?.id === category.id) edit(null);
  } catch (value) {
    error.value = toAppError(value).message;
    removing.value = null;
  }
}
</script>
<template>
  <DialogFrame title="分类管理" :busy="store.busy" @close="emit('close')">
    <p class="muted small">删除分类会清空任务分类，不会删除任务。</p>
    <div class="category-list">
      <div v-for="category in store.categories" :key="category.id" class="category-item">
        <span class="category-tag"
          ><i :style="{ backgroundColor: category.color }" />{{ category.name }}</span
        >
        <div>
          <AppButton
            variant="ghost"
            :disabled="store.busy"
            :aria-label="`编辑分类：${category.name}`"
            @click="edit(category)"
            >编辑</AppButton
          ><AppButton
            variant="ghost"
            :disabled="store.busy"
            :aria-label="`删除分类：${category.name}`"
            @click="removing = category"
            >删除</AppButton
          >
        </div>
      </div>
    </div>
    <form class="task-form" @submit.prevent="save">
      <h3>{{ selected ? '编辑分类' : '新建分类' }}</h3>
      <label>分类名称<input v-model="name" maxlength="100" required :disabled="store.busy" /></label
      ><label>分类颜色<input v-model="color" type="color" :disabled="store.busy" /></label>
      <p v-if="error" role="alert" class="form-error">{{ error }}</p>
      <div class="dialog-actions">
        <AppButton v-if="selected" :disabled="store.busy" @click="edit(null)">改为新建</AppButton
        ><AppButton type="submit" variant="primary" :disabled="store.busy">{{
          store.busy ? '正在保存…' : '保存分类'
        }}</AppButton>
      </div>
    </form>
  </DialogFrame>
  <ConfirmDialog
    v-if="removing"
    title="删除分类"
    :message="`删除“${removing.name}”？任务会保留，分类引用将被清空。`"
    :busy="store.busy"
    confirm-label="删除分类"
    @cancel="removing = null"
    @confirm="remove(removing!)"
  />
</template>
