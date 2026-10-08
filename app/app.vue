<script setup lang="ts">
import 'vue-sonner/style.css'

const view = useView()
</script>

<template>
  <SidebarProvider>
    <AppSidebar />
    <!-- Workspace всегда смонтирован: при уходе на другую страницу терминалы и агенты продолжают работать -->
    <Workspace :hidden="view !== 'workspace'" />
    <SettingsPage v-if="view === 'settings'" />
    <IntegrationPage v-else-if="view !== 'workspace'" :id="view" />
  </SidebarProvider>
  <Toaster theme="dark" rich-colors />
</template>
