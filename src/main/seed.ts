import type { FlowContent } from "../diagrams"

export interface SeedFlow {
  name: string
  contents: SeedContent[]
}

type SeedContent = Pick<FlowContent, "id" | "parentId" | "kind" | "name">
  & Partial<Pick<FlowContent, "body" | "status">>

export const starterFlows: SeedFlow[] = [
  {
    name: "Creative review",
    contents: [
      { id: 2, parentId: null, kind: "goal", name: "Подготовить креатив к публикации", status: "approved" },
      { id: 3, parentId: 2, kind: "task", name: "Собрать первый концепт по брифу", status: "accepted" },
      { id: 4, parentId: 3, kind: "solution", name: "Первый концепт", status: "review" },
      { id: 5, parentId: 4, kind: "comment", name: "По итогам ревью уточнить заголовок" },
      { id: 6, parentId: 3, kind: "solution", name: "Доработанный креатив готов к публикации", status: "accepted" },
    ],
  },
  {
    name: "Asset handoff",
    contents: [
      { id: 2, parentId: null, kind: "goal", name: "Передать утверждённые материалы продюсеру", status: "approved" },
      { id: 3, parentId: 2, kind: "task", name: "Загрузить финальные файлы в библиотеку", status: "accepted" },
      { id: 4, parentId: 3, kind: "solution", name: "Финальная версия доступна в библиотеке", status: "accepted" },
      { id: 5, parentId: 2, kind: "task", name: "Запросить утверждённую версию из библиотеки", status: "accepted" },
      { id: 6, parentId: 5, kind: "solution", name: "Продюсер получил утверждённую версию", status: "accepted" },
    ],
  },
  {
    name: "Production states",
    contents: [
      { id: 2, parentId: null, kind: "goal", name: "Провести материал от черновика до публикации", status: "approved" },
      { id: 3, parentId: 2, kind: "task", name: "Подготовить черновик и пройти ревью", status: "accepted" },
      { id: 4, parentId: 3, kind: "solution", name: "Черновик на ревью", status: "review" },
      { id: 5, parentId: 4, kind: "comment", name: "Внести правки по итогам ревью" },
      { id: 6, parentId: 3, kind: "solution", name: "Утверждённая версия", status: "accepted" },
      { id: 7, parentId: 2, kind: "task", name: "Опубликовать утверждённую версию", status: "accepted" },
      { id: 8, parentId: 7, kind: "solution", name: "Материал опубликован", status: "accepted" },
    ],
  },
]

export const contextualDevelopmentFlow: SeedFlow = {
  name: "Разработка Contextual",
  contents: [
    { id: 2, parentId: null, kind: "goal", name: "Перезапустить проект", body: "Рабочее пространство для контент-креаторов.", status: "approved" },
    { id: 3, parentId: 2, kind: "task", name: "Создать ветку TypeScript и настроить Electron Forge и Vite", status: "accepted" },
    { id: 4, parentId: 3, kind: "solution", name: "Настольное приложение запускается в dev-режиме", status: "accepted" },
    { id: 5, parentId: 4, kind: "evidence", name: "Проверена сборка Linux", body: "Команда `npm run build`, коммит `afe05f7`." },
    { id: 6, parentId: null, kind: "goal", name: "Открывать локальную библиотеку схем", status: "approved" },
    { id: 7, parentId: 6, kind: "task", name: "Настроить хранение схем в SQLite и заполнить базу примерами", status: "accepted" },
    { id: 8, parentId: 7, kind: "solution", name: "Примеры появляются при первом запуске и сохраняются между запусками", status: "accepted" },
    { id: 9, parentId: 8, kind: "evidence", name: "Повторный запуск не дублирует пример", body: "Миграция v1 → v2 добавила одну схему. Коммит `4c6b843`." },
    { id: 10, parentId: null, kind: "goal", name: "Просматривать схемы", status: "approved" },
    { id: 11, parentId: 10, kind: "task", name: "Подключить Pele, Nano Stores и nanotags", status: "accepted" },
    { id: 12, parentId: 11, kind: "solution", name: "Просмотр схем с обработкой ошибок отрисовки", status: "accepted" },
    { id: 13, parentId: 12, kind: "evidence", name: "Скриншот из Linux-сборки", body: "На скриншоте Pele отрисовал 20 узлов." },
    { id: 14, parentId: null, kind: "goal", name: "Открывать каждую схему по адресу", status: "approved" },
    { id: 15, parentId: 14, kind: "task", name: "Подключить Nano Stores Router и настроить адреса схем", body: "Маршрут `/diagrams/:id` и протокол `app://`.", status: "accepted" },
    { id: 16, parentId: 15, kind: "solution", name: "Прямое открытие, перезагрузка и переходы по истории работают", status: "accepted" },
    { id: 17, parentId: 16, kind: "evidence", name: "Адрес схемы сохранился после перезагрузки", body: "Маршрут `/diagrams/2` открылся напрямую. Коммит `aeb17c9`." },
    { id: 18, parentId: null, kind: "goal", name: "Работать с вариантами контента на схеме", status: "draft" },
    { id: 19, parentId: 18, kind: "comment", name: "Обсудили выбор узла и действия для выбранного узла" },
    { id: 20, parentId: 18, kind: "task", name: "Связать действия с агентом", status: "planned" },
    { id: 21, parentId: 18, kind: "task", name: "Добавить ветки и версии контента", status: "planned" },
  ],
}

export const wildberriesFlow: SeedFlow = {
  name: "Подготовка карточки Wildberries",
  contents: [
    { id: 2, parentId: null, kind: "goal", name: "Подготовить первый слайд карточки WB", status: "draft" },
    { id: 3, parentId: 2, kind: "task", name: "Собрать макет с ветровкой", status: "review" },
    { id: 4, parentId: 3, kind: "comment", name: "Убрать логотип с флага" },
    { id: 5, parentId: 3, kind: "solution", name: "Макет с крупным заголовком", status: "review" },
    { id: 6, parentId: 5, kind: "evidence", name: "Предпросмотр в размере мобильной выдачи" },
  ],
}
