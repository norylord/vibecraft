<script setup lang="ts">
import 'vue-sonner/style.css'

const view = useView()
const { isDark } = useSettings()
applyAppearance()
</script>

<template>
  <!-- h-svh: без него колонка растёт под контент и внутренние overflow-auto не скроллятся -->
  <SidebarProvider class="h-svh">
    <AppSidebar />
    <!-- Workspace всегда смонтирован: при уходе на другую страницу терминалы и агенты продолжают работать -->
    <Workspace :hidden="view !== 'workspace'" />
    <SettingsPage v-if="view === 'settings'" />
    <YouTrackPage v-else-if="view === 'youtrack'" />
    <GitLabPage v-else-if="view === 'gitlab'" />
  </SidebarProvider>
  <Toaster :theme="isDark ? 'dark' : 'light'" rich-colors />
</template>
