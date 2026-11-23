import { net, protocol } from "electron"
import { join } from "node:path"
import { pathToFileURL } from "node:url"

export const rendererURL = "app://contextual/"

export function registerRendererScheme() {
  protocol.registerSchemesAsPrivileged([
    { scheme: "app", privileges: { standard: true, secure: true } },
  ])
}

export function serveRenderer(directory: string) {
  protocol.handle("app", (request) => {
    const url = new URL(request.url)
    if (url.host !== "contextual" || request.method !== "GET") {
      return new Response(null, { status: 404 })
    }

    // Only load Vite's flat asset output or the SPA document from the bundle.
    const asset = /^\/assets\/([\w.-]+\.[\w]+)$/.exec(url.pathname)
    if (!asset && url.pathname.startsWith("/assets/")) {
      return new Response(null, { status: 404 })
    }

    const file = asset ? join("assets", asset[1]) : "index.html"
    return net.fetch(pathToFileURL(join(directory, file)).href)
  })
}
