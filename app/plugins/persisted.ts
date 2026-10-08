// Состояние из ~/.diogen/state.json должно быть загружено до первого рендера
export default defineNuxtPlugin(async () => {
  await loadPersisted()
})
