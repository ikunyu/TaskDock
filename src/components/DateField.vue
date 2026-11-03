<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, useId, watch } from 'vue';
import { localDate, validDate } from '../domain/tasks';
import AppIcon from './AppIcon.vue';

const props = withDefaults(
  defineProps<{
    modelValue: string | null;
    label: string;
    min?: string;
    max?: string;
    disabled?: boolean;
    required?: boolean;
  }>(),
  { min: '0001-01-01', max: '9999-12-31' },
);
const emit = defineEmits<{ 'update:modelValue': [value: string | null] }>();
const id = useId();
const root = ref<HTMLElement>(),
  toggle = ref<HTMLButtonElement>();
const open = ref(false),
  year = ref(new Date().getFullYear()),
  month = ref(new Date().getMonth() + 1);
const shortcuts = [
  { days: 0, label: '今天' },
  { days: 1, label: '明天' },
  { days: 7, label: '一周后' },
];
function dateAt(y: number, m: number, day: number) {
  const value = new Date(0);
  value.setFullYear(y, m - 1, day);
  value.setHours(12, 0, 0, 0);
  return value;
}
function relative(days: number) {
  const value = new Date();
  value.setDate(value.getDate() + days);
  return localDate(value);
}
function allowed(value: string) {
  return validDate(value) && value >= props.min && value <= props.max;
}
const minYear = computed(() => Number(props.min.slice(0, 4)));
const maxYear = computed(() => Number(props.max.slice(0, 4)));
function show(y: number, m: number) {
  year.value = Math.min(maxYear.value, Math.max(minYear.value, y));
  const first = year.value === minYear.value ? Number(props.min.slice(5, 7)) : 1;
  const last = year.value === maxYear.value ? Number(props.max.slice(5, 7)) : 12;
  month.value = Math.min(last, Math.max(first, m));
}
function toggleCalendar() {
  if (props.disabled) return;
  if (!open.value) {
    const base = validDate(props.modelValue ?? '') ? props.modelValue! : relative(0);
    show(Number(base.slice(0, 4)), Number(base.slice(5, 7)));
  }
  open.value = !open.value;
}
function canShift(delta: number) {
  const serial = year.value * 12 + month.value - 1 + delta;
  const lower = minYear.value * 12 + Number(props.min.slice(5, 7)) - 1;
  const upper = maxYear.value * 12 + Number(props.max.slice(5, 7)) - 1;
  return serial >= lower && serial <= upper;
}
function shift(delta: number) {
  if (!canShift(delta)) return;
  const serial = year.value * 12 + month.value - 1 + delta;
  show(Math.floor(serial / 12), (serial % 12) + 1);
}
function changeYear(event: Event) {
  const input = event.target as HTMLInputElement;
  const value = Number(input.value);
  if (Number.isInteger(value) && value > 0) show(value, month.value);
  input.value = String(year.value);
}
const cells = computed(() => {
  const first = dateAt(year.value, month.value, 1);
  const leading = (first.getDay() + 6) % 7;
  const count = dateAt(year.value, month.value + 1, 0).getDate();
  return Array.from({ length: Math.ceil((leading + count) / 7) * 7 }, (_, index) => {
    const day = index - leading + 1;
    return day < 1 || day > count
      ? null
      : { day, date: localDate(dateAt(year.value, month.value, day)) };
  });
});
function select(value: string) {
  if (props.disabled || !allowed(value)) return;
  emit('update:modelValue', value);
  open.value = false;
  void nextTick(() => toggle.value?.focus());
}
function escape(event: KeyboardEvent) {
  if (!open.value || event.key !== 'Escape') return;
  event.preventDefault();
  event.stopPropagation();
  open.value = false;
  toggle.value?.focus();
}
function outside(event: PointerEvent) {
  if (event.target instanceof Node && !root.value?.contains(event.target)) open.value = false;
}
watch(
  () => props.disabled,
  (disabled) => {
    if (disabled) open.value = false;
  },
);
onMounted(() => document.addEventListener('pointerdown', outside));
onUnmounted(() => document.removeEventListener('pointerdown', outside));
</script>

<template>
  <div ref="root" class="date-field" @keydown="escape">
    <label :for="id">{{ label }}</label>
    <div class="date-input-row">
      <input
        :id="id"
        type="date"
        class="date-field-input"
        :value="modelValue ?? ''"
        :min="min"
        :max="max"
        :required="required"
        :disabled="disabled"
        @input="emit('update:modelValue', ($event.target as HTMLInputElement).value || null)"
      />
      <button
        ref="toggle"
        type="button"
        class="date-calendar-toggle"
        :disabled="disabled"
        :aria-label="`选择${label}`"
        :aria-expanded="open"
        :aria-controls="`${id}-calendar`"
        @click="toggleCalendar"
      >
        <AppIcon name="plan" :size="16" />
      </button>
    </div>
    <div class="date-shortcuts" :aria-label="`${label}快捷选择`">
      <button
        v-for="item in shortcuts"
        :key="item.days"
        type="button"
        :disabled="disabled || !allowed(relative(item.days))"
        @click="select(relative(item.days))"
      >
        {{ item.label }}
      </button>
    </div>
    <div
      v-if="open"
      :id="`${id}-calendar`"
      class="date-calendar"
      role="group"
      :aria-label="`${label}日历`"
    >
      <div class="date-calendar-toolbar">
        <button
          type="button"
          :disabled="disabled || !canShift(-1)"
          aria-label="上个月"
          @click="shift(-1)"
        >
          ‹
        </button>
        <input
          class="date-year"
          type="number"
          :value="year"
          :min="minYear"
          :max="maxYear"
          :aria-label="`${label}年份`"
          :disabled="disabled"
          @change="changeYear"
          @keydown.enter.prevent="changeYear"
        />
        <select
          :value="month"
          :aria-label="`${label}月份`"
          :disabled="disabled"
          @change="show(year, Number(($event.target as HTMLSelectElement).value))"
        >
          <option
            v-for="m in 12"
            :key="m"
            :value="m"
            :disabled="
              (year === minYear && m < Number(min.slice(5, 7))) ||
              (year === maxYear && m > Number(max.slice(5, 7)))
            "
          >
            {{ m }}月
          </option>
        </select>
        <button
          type="button"
          :disabled="disabled || !canShift(1)"
          aria-label="下个月"
          @click="shift(1)"
        >
          ›
        </button>
      </div>
      <div class="date-calendar-grid">
        <span
          v-for="day in ['一', '二', '三', '四', '五', '六', '日']"
          :key="day"
          class="date-weekday"
          >{{ day }}</span
        >
        <template v-for="(cell, index) in cells" :key="index"
          ><button
            v-if="cell"
            type="button"
            :disabled="disabled || !allowed(cell.date)"
            :aria-label="cell.date"
            :aria-pressed="modelValue === cell.date"
            :class="{ 'date-today': cell.date === relative(0) }"
            @click="select(cell.date)"
          >
            {{ cell.day }}</button
          ><span v-else
        /></template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.date-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
  font-size: 12px;
  color: var(--muted);
}
.date-input-row {
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
}
.date-field .date-field-input {
  flex: 1;
  width: 0;
  min-width: 0;
}
.date-field-input::-webkit-calendar-picker-indicator {
  display: none;
}
.date-field input,
.date-field select {
  border: 1px solid var(--border);
  border-radius: 7px;
  background: var(--canvas);
  color: var(--text);
  padding: 8px;
  min-width: 0;
  font: inherit;
}
.date-field button {
  border: 1px solid var(--border);
  border-radius: 5px;
  background: var(--surface);
  color: var(--muted);
  padding: 3px 6px;
  font: inherit;
}
.date-field button:hover:not(:disabled) {
  background: var(--accent-soft);
  color: var(--accent);
}
.date-calendar-toggle {
  display: inline-grid;
  place-items: center;
  flex: 0 0 30px;
  height: 34px;
}
.date-shortcuts {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}
.date-shortcuts button {
  font-size: 10px;
}
.date-calendar {
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--surface);
  padding: 8px;
}
.date-calendar-toolbar {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 4px;
  margin-bottom: 8px;
}
.date-calendar .date-year {
  width: 76px;
  padding: 4px;
}
.date-calendar select {
  flex: 1;
  width: auto;
  padding: 4px;
}
.date-calendar-grid {
  display: grid;
  grid-template-columns: repeat(7, minmax(0, 1fr));
  gap: 2px;
  text-align: center;
}
.date-calendar-grid button {
  min-width: 0;
  min-height: 25px;
  padding: 2px;
  border-color: transparent;
  font-size: 11px;
}
.date-weekday {
  padding: 2px 0 5px;
  font-size: 10px;
}
.date-calendar-grid .date-today {
  border-color: var(--accent);
}
.date-calendar-grid button[aria-pressed='true'] {
  color: var(--accent);
  background: var(--accent-soft);
}
</style>
