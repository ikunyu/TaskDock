<script setup lang="ts">
import { useAppStore } from '../stores/app';
import type { ThemePreference } from '../types';
const props = defineProps<{ id?: string; disabled?: boolean }>();
const store = useAppStore();
const options: { value: ThemePreference; label: string }[] = [
  { value: 'system', label: '跟随系统' },
  { value: 'light', label: '明亮' },
  { value: 'dark', label: '深色' },
];
</script>
<template>
  <div class="theme-select">
    <label :for="props.id ?? 'appearance-theme'">外观</label>
    <select
      :id="props.id ?? 'appearance-theme'"
      :value="store.theme"
      :disabled="store.busy || props.disabled"
      @change="store.changeTheme(($event.target as HTMLSelectElement).value as ThemePreference)"
    >
      <option v-for="option in options" :key="option.value" :value="option.value">
        {{ option.label }}
      </option>
    </select>
  </div>
</template>
