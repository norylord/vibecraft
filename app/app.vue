<script setup lang="ts">
import 'vue-sonner/style.css'

const view = useView()
</script>

<template>
  <!-- h-svh: без него колонка растёт под контент и внутренние overflow-auto не скроллятся -->
  <SidebarProvider class="h-svh">
    <AppSidebar />
    <!-- Workspace всегда смонтирован: при уходе на другую страницу терминалы и агенты продолжают работать -->
    <Workspace :hidden="view !== 'workspace'" />
    <SettingsPage v-if="view === 'settings'" />
    <YouTrackPage v-else-if="view === 'youtrack'" />
    <IntegrationPage v-else-if="view !== 'workspace'" :id="view" />
  </SidebarProvider>
  <Toaster theme="dark" rich-colors />
</template>
