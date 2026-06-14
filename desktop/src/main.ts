import { createApp } from "vue";
import { createPinia } from "pinia";

import App from "./App.vue";
import { router } from "./router";
import { useSettingsStore } from "./stores/settings";
import "./styles/global.css";

const app = createApp(App);
const pinia = createPinia();
app.use(pinia);

// Грузим настройки до монтирования, чтобы guard сразу знал, настроено ли
// подключение, и не моргал лишним редиректом на /settings.
const settings = useSettingsStore(pinia);
settings.load().finally(() => {
  app.use(router);
  app.mount("#app");
});
