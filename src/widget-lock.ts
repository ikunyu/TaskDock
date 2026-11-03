import { createApp } from 'vue';
import WidgetLockButton from './components/WidgetLockButton.vue';
import './styles/theme.css';

// Button-only entry: no task store, scheduler, updater or application chrome.
createApp(WidgetLockButton).mount('#app');
