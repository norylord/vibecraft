<script setup lang="ts">
import { useIntervalFn } from '@vueuse/core'
import 'vue-sonner/style.css'

const view = useView()
const { isDark } = useSettings()
applyAppearance()

// При старте и раз в 6 часов — Vibecraft держат открытым днями. В dev нечего обновлять: нет .app
const { checkUpdate } = useUpdater()
if (!import.meta.dev) useIntervalFn(checkUpdate, 6 * 3600_000, { immediateCallback: true })
</script>

<template>
  <!-- h-svh: без него колонка растёт под контент и внутренние overflow-auto не скроллятся; pb-7 — место под футер -->
  <SidebarProvider class="h-svh pb-7">
    <AppSidebar />
    <!-- Workspace всегда смонтирован: при уходе на другую страницу терминалы и агенты продолжают работать -->
    <Workspace :hidden="view !== 'workspace'" />
    <SettingsPage v-if="view === 'settings'" />
    <YouTrackPage v-else-if="view === 'youtrack'" />
    <GitLabPage v-else-if="view === 'gitlab'" />
    <RepoPage v-else-if="view === 'repo'" />
    <AppFooter />
  </SidebarProvider>
  <Toaster :theme="isDark ? 'dark' : 'light'" rich-colors />
  <UpdateDialog />
</template>
