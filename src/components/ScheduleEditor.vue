<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import type { ScheduleConfig, Recurrence, ReminderRepeat } from '../schedule-types';
import { emptySchedule } from '../schedule-types';
import { toLocalDateTime, toUtcDateTime } from '../domain/tasks';
import DateField from './DateField.vue';
const props = withDefaults(
  defineProps<{
    modelValue: ScheduleConfig;
    busy?: boolean;
    hasDue: boolean;
    minimumDate?: string;
  }>(),
  { minimumDate: '0001-01-01' },
);
const emit = defineEmits<{ 'update:modelValue': [value: ScheduleConfig] }>();
function update(patch: Partial<ScheduleConfig>) {
  emit('update:modelValue', { ...props.modelValue, ...patch });
}
const localTime = ref(toLocalDateTime(props.modelValue.at));
const timeInput = ref<HTMLInputElement>();
const timeError = ref('');
watch(
  () => props.modelValue.at,
  (at) => {
    localTime.value = toLocalDateTime(at);
    timeError.value = '';
    timeInput.value?.setCustomValidity('');
  },
);
function setTime(event: Event) {
  const input = event.target as HTMLInputElement;
  try {
    update({ at: localTime.value ? toUtcDateTime(localTime.value) : null });
    timeError.value = '';
    input.setCustomValidity('');
  } catch {
    timeError.value = '本地时间无效，请重新选择。';
    input.setCustomValidity(timeError.value);
  }
}
function cancelReminders() {
  update({ ...emptySchedule(), recurrence: props.modelValue.recurrence });
  localTime.value = '';
  timeError.value = '';
  timeInput.value?.setCustomValidity('');
}
function setReminderRepeat(patch: Partial<ReminderRepeat>) {
  update({
    reminderRepeat: { interval: 1, unit: 'days', ...props.modelValue.reminderRepeat, ...patch },
  });
}
const maxReminderInterval = computed(() => {
  const unit = props.modelValue.reminderRepeat?.unit;
  return unit === 'minutes' ? 5256000 : unit === 'hours' ? 87600 : 3650;
});
const frequency = computed(() => props.modelValue.recurrence?.frequency ?? 'none');
function setFrequency(event: Event) {
  const f = (event.target as HTMLSelectElement).value;
  update({
    recurrence:
      f === 'none'
        ? null
        : {
            frequency: f as Recurrence['frequency'],
            interval: 1,
            weekdays: [new Date().getDay() === 0 ? 6 : new Date().getDay() - 1],
            monthDay: new Date().getDate(),
            until: null,
          },
  });
}
function repeat(patch: Partial<Recurrence>) {
  if (props.modelValue.recurrence)
    update({ recurrence: { ...props.modelValue.recurrence, ...patch } });
}
function toggleDay(d: number) {
  const days = props.modelValue.recurrence?.weekdays ?? [];
  repeat({ weekdays: days.includes(d) ? days.filter((v) => v !== d) : [...days, d].sort() });
}
function before(m: number) {
  const list = props.modelValue.beforeMinutes;
  update({ beforeMinutes: list.includes(m) ? list.filter((v) => v !== m) : [...list, m] });
}
</script>
<template>
  <fieldset class="schedule-editor" :disabled="busy">
    <legend>提醒与重复</legend>
    <label
      >{{ modelValue.reminderRepeat ? '首次提醒（本地时间）' : '指定时间提醒（本地时间）'
      }}<input
        ref="timeInput"
        v-model="localTime"
        type="datetime-local"
        :required="!!modelValue.reminderRepeat"
        :min="`${minimumDate}T00:00`"
        max="9999-12-31T23:59"
        @change="setTime"
    /></label>
    <p v-if="timeError" class="form-error" role="alert">{{ timeError }}</p>
    <label class="schedule-check"
      ><input
        type="checkbox"
        :checked="!!modelValue.reminderRepeat"
        @change="
          update({
            reminderRepeat: modelValue.reminderRepeat ? null : { interval: 1, unit: 'days' },
          })
        "
      />定期提醒同一任务</label
    >
    <template v-if="modelValue.reminderRepeat">
      <div class="schedule-options reminder-presets">
        <button
          v-for="preset in [
            { interval: 30, unit: 'minutes', label: '每30分钟' },
            { interval: 1, unit: 'hours', label: '每小时' },
            { interval: 6, unit: 'hours', label: '每6小时' },
            { interval: 1, unit: 'days', label: '每天' },
            { interval: 7, unit: 'days', label: '每周' },
          ] as const"
          :key="preset.label"
          type="button"
          :aria-pressed="
            modelValue.reminderRepeat.interval === preset.interval &&
            modelValue.reminderRepeat.unit === preset.unit
          "
          @click="setReminderRepeat({ interval: preset.interval, unit: preset.unit })"
        >
          {{ preset.label }}
        </button>
      </div>
      <div class="form-grid">
        <label
          >每隔<input
            type="number"
            min="1"
            :max="maxReminderInterval"
            step="1"
            required
            :value="modelValue.reminderRepeat.interval"
            @input="
              setReminderRepeat({ interval: Number(($event.target as HTMLInputElement).value) })
            "
        /></label>
        <label
          >时间单位<select
            :value="modelValue.reminderRepeat.unit"
            @change="
              setReminderRepeat({
                unit: ($event.target as HTMLSelectElement).value as ReminderRepeat['unit'],
              })
            "
          >
            <option value="minutes">分钟</option>
            <option value="hours">小时</option>
            <option value="days">天</option>
          </select></label
        >
      </div>
      <p class="muted small">
        从首次提醒开始，按此间隔提醒同一任务；完成、删除或到达截止时间后停止。软件关闭期间不发送，重新打开仅合并补发最近一次。
      </p>
    </template>
    <label class="schedule-check"
      ><input
        type="checkbox"
        :checked="modelValue.atDeadline"
        :disabled="!hasDue"
        @change="update({ atDeadline: !modelValue.atDeadline })"
      />截止时提醒</label
    >
    <div class="schedule-options">
      <label
        v-for="[m, text] in [
          [10, '提前10分钟'],
          [30, '提前30分钟'],
          [60, '提前1小时'],
          [1440, '提前1天'],
        ] as const"
        :key="m"
        class="schedule-check"
        ><input
          type="checkbox"
          :checked="modelValue.beforeMinutes.includes(m)"
          :disabled="!hasDue"
          @change="before(m)"
        />{{ text }}</label
      >
    </div>
    <p class="muted small">
      选择“仅日期”时，按截止日期当天23:59计算截止和提前提醒。系统通知被禁用时可在提醒中心查看。
    </p>
    <button type="button" @click="cancelReminders">取消所有提醒</button>
    <label
      >任务重复周期<select :value="frequency" @change="setFrequency">
        <option value="none">不重复</option>
        <option value="daily">每天</option>
        <option value="weekly">每周</option>
        <option value="monthly">每月</option>
        <option value="interval">自定义天数间隔</option>
      </select></label
    >
    <template v-if="modelValue.recurrence">
      <label
        >间隔（{{ frequency === 'weekly' ? '周' : frequency === 'monthly' ? '月' : '天' }}）<input
          type="number"
          min="1"
          max="3650"
          :value="modelValue.recurrence.interval"
          @input="repeat({ interval: Number(($event.target as HTMLInputElement).value) })"
          required
      /></label>
      <div v-if="frequency === 'weekly'" class="schedule-options">
        <label
          v-for="(name, d) in ['周一', '周二', '周三', '周四', '周五', '周六', '周日']"
          :key="d"
          class="schedule-check"
          ><input
            type="checkbox"
            :checked="modelValue.recurrence.weekdays.includes(d)"
            @change="toggleDay(d)"
          />{{ name }}</label
        >
      </div>
      <label v-if="frequency === 'monthly'"
        >每月几号<input
          type="number"
          min="1"
          max="31"
          :value="modelValue.recurrence.monthDay"
          @input="repeat({ monthDay: Number(($event.target as HTMLInputElement).value) })"
          required
      /></label>
      <DateField
        label="结束日期（留空无限）"
        :model-value="modelValue.recurrence.until"
        :min="minimumDate"
        :disabled="busy"
        @update:model-value="repeat({ until: $event })"
      />
      <p class="muted small">
        此项会生成新的任务，与上方定期提醒不同。重复任务需要截止日期；所选月份无该日期时取最后一天。完成此任务不影响下一次。
      </p>
    </template>
  </fieldset>
</template>
