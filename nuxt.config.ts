import tailwindcss from '@tailwindcss/vite'

// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: '2025-07-15',
  devtools: { enabled: true },
  telemetry: false,
  // Tauri раздаёт статику, сервера нет
  ssr: false,
  app: {
    head: {
      title: 'Diogen',
      htmlAttrs: { lang: 'ru', class: 'dark' },
    },
  },
  modules: ['shadcn-nuxt'],
  shadcn: {
    prefix: '',
    componentDir: '@/components/ui',
  },
  css: ['~/assets/css/tailwind.css'],
  // Должен совпадать с build.devUrl в src-tauri/tauri.conf.json
  devServer: { port: 1420 },
  vite: {
    plugins: [tailwindcss()],
    clearScreen: false,
    envPrefix: ['VITE_', 'TAURI_'],
    server: { strictPort: true },
  },
  ignore: ['**/src-tauri/**'],
})
