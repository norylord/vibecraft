<script setup lang="ts">
import 'vue-sonner/style.css'
</script>

<template>
  <SidebarProvider>
    <AppSidebar />
    <Workspace />
  </SidebarProvider>
  <Toaster theme="dark" rich-colors />
</template>
