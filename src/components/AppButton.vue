<script setup lang="ts">
withDefaults(
  defineProps<{
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
    disabled?: boolean;
    type?: 'button' | 'submit';
  }>(),
  {
    variant: 'secondary',
    disabled: false,
    type: 'button',
  },
);
</script>
<template>
  <button :type="type" :disabled="disabled" class="app-button" :class="`button-${variant}`">
    <slot />
  </button>
</template>
