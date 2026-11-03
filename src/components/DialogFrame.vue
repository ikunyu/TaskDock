<script setup lang="ts">
import { onMounted, onUnmounted, ref, useId } from 'vue';
import AppButton from './AppButton.vue';
import AppIcon from './AppIcon.vue';
const props = defineProps<{ title: string; drawer?: boolean; busy?: boolean }>();
const emit = defineEmits<{ close: [] }>();
const headingId = useId();
const panel = ref<HTMLElement>();
let previous: Element | null = null;
function visible(element: HTMLElement) {
  // Visibility should not depend on layout rectangles (e.g. a hidden settings
  // page, or an element before its first layout pass).
  for (let current: HTMLElement | null = element; current; current = current.parentElement) {
    const style = getComputedStyle(current);
    if (
      current.hidden ||
      current.hasAttribute('inert') ||
      style.display === 'none' ||
      style.visibility === 'hidden'
    )
      return false;
  }
  return true;
}
function keydown(event: KeyboardEvent) {
  const dialogs = document.querySelectorAll('[data-taskdock-dialog]');
  if (dialogs.item(dialogs.length - 1) !== panel.value) return;
  if (event.key === 'Escape') {
    event.stopPropagation();
    event.preventDefault();
    if (!props.busy) emit('close');
  }
  if (event.key !== 'Tab') return;
  const elements = Array.from(
    panel.value?.querySelectorAll<HTMLElement>(
      'button:not(:disabled),input:not(:disabled),textarea:not(:disabled),select:not(:disabled),[tabindex="0"]',
    ) ?? [],
  ).filter(visible);
  const first = elements[0],
    last = elements.at(-1);
  if (!panel.value?.contains(document.activeElement)) {
    event.preventDefault();
    ((event.shiftKey ? last : first) ?? panel.value)?.focus();
  } else if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last?.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first?.focus();
  }
}
onMounted(() => {
  window.addEventListener('keydown', keydown);
  previous = document.activeElement;
  panel.value?.querySelector<HTMLElement>('button')?.focus();
});
onUnmounted(() => {
  window.removeEventListener('keydown', keydown);
  if (previous instanceof HTMLElement && previous.isConnected) previous.focus();
});
</script>
<template>
  <Teleport to="body">
    <div
      class="dialog-backdrop"
      :class="{ 'drawer-backdrop': drawer }"
      @click.self="!busy && emit('close')"
    >
      <section
        ref="panel"
        tabindex="-1"
        data-taskdock-dialog
        :class="drawer ? 'detail-drawer' : 'dialog-panel'"
        role="dialog"
        aria-modal="true"
        :aria-labelledby="headingId"
      >
        <header class="section-heading dialog-heading">
          <h2 :id="headingId">{{ title }}</h2>
          <AppButton
            variant="ghost"
            :disabled="busy"
            :aria-label="`关闭${title}`"
            @click="emit('close')"
            ><AppIcon name="close"
          /></AppButton>
        </header>
        <slot />
      </section>
    </div>
  </Teleport>
</template>
