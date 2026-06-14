<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useSettingsStore } from "./stores/settings";
import { useUpdaterStore } from "./stores/updater";
import ToastHost from "./components/ToastHost.vue";
import UpdateBanner from "./components/UpdateBanner.vue";

const router = useRouter();
const settings = useSettingsStore();
const updater = useUpdaterStore();

// Проверяем обновления на старте и далее раз в час.
onMounted(() => updater.startAuto());

const navItems = computed(() =>
  router.options.routes
    .filter((r) => r.name)
    .map((r) => ({
      name: r.name as string,
      title: (r.meta?.title as string) ?? (r.name as string),
      icon: (r.meta?.icon as string) ?? "🐾",
    })),
);

// Короткий хост из base URL для подписи в сайдбаре.
const host = computed(() => {
  try {
    return new URL(settings.baseUrl).host;
  } catch {
    return settings.baseUrl || "не настроено";
  }
});
</script>

<template>
  <div class="shell">
    <aside class="sidebar">
      <div class="brand">
        <img src="/corgi.png" alt="" class="brand-logo" />
        <div class="brand-text">
          <strong>CorgiTrack</strong>
          <span>дневник питомца</span>
        </div>
      </div>

      <nav class="nav">
        <RouterLink
          v-for="item in navItems"
          :key="item.name"
          :to="{ name: item.name }"
          class="nav-link"
          active-class="active"
        >
          <span class="nav-ico">{{ item.icon }}</span>
          <span>{{ item.title }}</span>
        </RouterLink>
      </nav>

      <div class="conn" :class="{ on: settings.configured }">
        <span class="dot" />
        <span class="conn-host">{{ host }}</span>
      </div>
    </aside>

    <main class="content">
      <RouterView v-slot="{ Component }">
        <Transition name="page" mode="out-in">
          <component :is="Component" />
        </Transition>
      </RouterView>
    </main>

    <UpdateBanner />
    <ToastHost />
  </div>
</template>

<style scoped>
.shell {
  display: grid;
  grid-template-columns: var(--sidebar-w) 1fr;
  height: 100%;
}

.sidebar {
  display: flex;
  flex-direction: column;
  gap: 1.4rem;
  padding: 1.4rem 1rem;
  background: linear-gradient(180deg, var(--surface), var(--surface-2));
  border-right: 1px solid rgba(201, 116, 43, 0.12);
  box-shadow: var(--shadow-sm);
}

.brand {
  display: flex;
  align-items: center;
  gap: 0.7rem;
  padding: 0.2rem 0.4rem;
}
.brand-logo {
  width: 46px;
  height: 46px;
  filter: drop-shadow(0 3px 6px rgba(201, 116, 43, 0.3));
}
.brand-text {
  display: flex;
  flex-direction: column;
  line-height: 1.1;
}
.brand-text strong {
  font-family: var(--font-display);
  font-size: 1.2rem;
  color: var(--ink);
}
.brand-text span {
  font-size: 0.72rem;
  color: var(--ink-soft);
}

.nav {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}
.nav-link {
  display: flex;
  align-items: center;
  gap: 0.7rem;
  padding: 0.65rem 0.85rem;
  border-radius: var(--r-md);
  color: var(--ink-soft);
  font-family: var(--font-display);
  font-weight: 500;
  transition:
    background 0.14s ease,
    color 0.14s ease,
    transform 0.14s ease;
}
.nav-link:hover {
  background: var(--corgi-wash);
  color: var(--ink);
  transform: translateX(2px);
}
.nav-link.active {
  background: linear-gradient(135deg, var(--corgi), var(--corgi-deep));
  color: #fff;
  box-shadow: 0 6px 14px rgba(201, 116, 43, 0.32);
}
.nav-ico {
  font-size: 1.15rem;
  width: 1.4rem;
  text-align: center;
}

.conn {
  margin-top: auto;
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.55rem 0.7rem;
  border-radius: var(--r-md);
  background: var(--paper-deep);
  font-size: 0.78rem;
  color: var(--ink-soft);
}
.conn .dot {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  background: var(--ink-faint);
  flex: none;
}
.conn.on .dot {
  background: var(--ok);
  box-shadow: 0 0 0 3px rgba(111, 168, 107, 0.25);
}
.conn-host {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.content {
  overflow: auto;
  padding: 2rem 2.4rem;
}
</style>
