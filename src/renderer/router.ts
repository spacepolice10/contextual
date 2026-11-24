import { createRouter } from "@nanostores/router"

export const $router = createRouter({
  library: "/",
  diagram: "/diagrams/:id",
  focused: "/diagrams/:id/focus/:nodeId",
})
