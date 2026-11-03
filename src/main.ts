import { createApp } from 'vue';
import { createPinia } from 'pinia';
import App from './App.vue';
import { reportError } from './services/logger';
import './styles/theme.css';
import './styles/app.css';
import './styles/tasks.css';

const app = createApp(App);
const pinia = createPinia();
app.use(pinia);
app.config.errorHandler = () => {
  reportError('VUE_ERROR');
  window.dispatchEvent(new Event('taskdock-ui-error'));
};
window.addEventListener('unhandledrejection', () => {
  reportError('UNHANDLED_REJECTION');
  window.dispatchEvent(new Event('taskdock-ui-error'));
});
window.addEventListener('error', () => {
  reportError('WINDOW_ERROR');
  window.dispatchEvent(new Event('taskdock-ui-error'));
});
app.mount('#app');
import './styles/desktop.css';

import './styles/schedule.css';

import './styles/data.css';
import './styles/release.css';
