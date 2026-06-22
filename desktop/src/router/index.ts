import { createRouter, createWebHashHistory } from "vue-router";
import { useSettingsStore } from "../stores/settings";

// Хеш-история — безопаснее для Tauri (нет сервера, который раздавал бы маршруты).
const Dashboard = () => import("../views/Dashboard.vue");
const Treatments = () => import("../views/Treatments.vue");
const VetBook = () => import("../views/VetBook.vue");
const Family = () => import("../views/Family.vue");
const Settings = () => import("../views/Settings.vue");

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", redirect: "/dashboard" },
    {
      path: "/dashboard",
      name: "dashboard",
      component: Dashboard,
      meta: { title: "Уведомления", icon: "🦴" },
    },
    {
      path: "/treatments",
      name: "treatments",
      component: Treatments,
      meta: { title: "Лечения", icon: "💊" },
    },
    {
      path: "/vetbook",
      name: "vetbook",
      component: VetBook,
      meta: { title: "Веткнига", icon: "📖" },
    },
    {
      path: "/family",
      name: "family",
      component: Family,
      meta: { title: "Семья", icon: "👪" },
    },
    {
      path: "/settings",
      name: "settings",
      component: Settings,
      meta: { title: "Настройки", icon: "⚙️" },
    },
  ],
});

// Пока подключение не настроено — пускаем только на экран настроек.
router.beforeEach((to) => {
  const settings = useSettingsStore();
  if (!settings.configured && to.name !== "settings") {
    return { name: "settings" };
  }
  return true;
});
