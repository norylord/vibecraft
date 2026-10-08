<script setup lang="ts">
import { FolderGit2Icon, SearchIcon } from '@lucide/vue'

const paletteOpen = useState('palette', () => false)
</script>

<template>
  <Sidebar variant="inset">
    <!-- pt-9: место под «светофор» macOS; пустая часть шапки таскает окно -->
    <SidebarHeader data-tauri-drag-region class="pt-9">
      <Button variant="outline" class="justify-start text-muted-foreground" @click="paletteOpen = true">
        <SearchIcon data-icon="inline-start" />
        Поиск и команды
        <KbdGroup class="ml-auto">
          <Kbd>⌘</Kbd>
          <Kbd>K</Kbd>
        </KbdGroup>
      </Button>
    </SidebarHeader>
    <SidebarContent>
      <SidebarGroup>
        <SidebarGroupLabel>Проекты</SidebarGroupLabel>
        <Empty>
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <FolderGit2Icon />
            </EmptyMedia>
            <EmptyTitle>Проектов пока нет</EmptyTitle>
            <EmptyDescription>Добавьте git-репозиторий, чтобы запускать агентов в worktree</EmptyDescription>
          </EmptyHeader>
        </Empty>
      </SidebarGroup>
    </SidebarContent>
  </Sidebar>
</template>
