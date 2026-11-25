// Historical seed sources used to identify unchanged examples during migration.
export const starterDiagrams = [
  {
    name: "Creative review",
    source: `flowchart LR
  brief["Creative brief"] --> concept["First concept"]
  concept --> review{"Review"}
  review -->|Revise| concept
  review -->|Approve| release["Ready to publish"]
  classDef highlight fill:#f2dfd1,stroke:#b96d49,color:#37291f
  class review highlight`,
  },
  {
    name: "Asset handoff",
    source: `sequenceDiagram
  actor Designer
  participant Library
  actor Producer
  Designer->>Library: Upload final assets
  Producer->>Library: Request approved assets
  Library-->>Producer: Return the approved version`,
  },
  {
    name: "Production states",
    source: `stateDiagram-v2
  [*] --> Draft
  Draft --> InReview
  InReview --> Draft: Changes requested
  InReview --> Approved
  Approved --> Published
  Published --> [*]`,
  },
]

export const contextualDevelopmentDiagramV2 = {
  name: "Разработка Contextual",
  source: `flowchart LR
  product["Contextual<br/>Рабочее пространство для контент-креаторов"]

  product --> foundation["Цель<br/>Перезапустить проект"]
  foundation --> bootstrap["Задачи<br/>Создать ветку для TypeScript<br/>Настроить Electron Forge и Vite"]
  bootstrap --> desktop["Готово<br/>Приложение запускается в dev-режиме<br/>Linux-сборка проверена"]

  product --> library["Цель<br/>Открывать локальную библиотеку схем"]
  library --> storage["Задачи<br/>Хранить исходники Mermaid в SQLite<br/>Заполнить базу примерами"]
  storage --> seeded["Готово<br/>Примеры появляются при первом запуске<br/>Данные сохраняются между запусками"]

  product --> viewer["Цель<br/>Просматривать схемы"]
  viewer --> rendering["Задачи<br/>Рендерить Mermaid через Pele<br/>Связать интерфейс с Nano Stores и nanotags"]
  rendering --> preview["Готово<br/>Просмотр схем и обработка ошибок рендера"]

  product --> navigation["Цель<br/>Открывать каждую схему по адресу"]
  navigation --> routing["Задачи<br/>Подключить Nano Stores Router<br/>Добавить /diagrams/:id и протокол app://"]
  routing --> verified["Готово<br/>Прямое открытие, перезагрузка<br/>и переходы назад и вперёд проверены"]

  product --> interaction["Цель<br/>Работать с вариантами контента на схеме"]
  interaction --> selection["Обсудили<br/>Выбор узла и действия для выбранного узла"]
  selection -.-> planned["Дальше<br/>Связать действия с агентом<br/>Добавить ветки и версии контента"]

  classDef goal fill:#eee4d7,stroke:#846c53,color:#30271f
  classDef done fill:#e5eee4,stroke:#667b63,color:#2c3b29
  classDef future fill:#f1e8e2,stroke:#96705d,color:#4c3023
  class product,foundation,library,viewer,navigation,interaction goal
  class desktop,seeded,preview,verified done
  class selection,planned future`,
}

export const contextualDevelopmentDiagram = {
  name: contextualDevelopmentDiagramV2.name,
  source: `${contextualDevelopmentDiagramV2.source}

  desktop --> buildEvidence["Подтверждение<br/>Сборка Linux: npm run build<br/>Коммит afe05f7"]
  seeded --> storageEvidence["Подтверждение<br/>Миграция v1 → v2 добавила одну схему<br/>Повторный запуск не создал дубль<br/>Коммит 4c6b843"]
  preview --> renderEvidence["Подтверждение<br/>Pele отрисовал 20 узлов<br/>Скриншот из Linux-сборки"]
  verified --> routeEvidence["Подтверждение<br/>/diagrams/2 открылась напрямую<br/>Перезагрузка сохранила адрес<br/>Коммит aeb17c9"]

  classDef evidence fill:#e6edf3,stroke:#657986,color:#293b49
  class buildEvidence,storageEvidence,renderEvidence,routeEvidence evidence`,
}
