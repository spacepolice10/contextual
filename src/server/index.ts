import { Hono } from "hono"
import { serve } from "@hono/node-server"
import { serveStatic } from "@hono/node-server/serve-static"
import { resolve } from "node:path"
import { createLibrary } from "../application/library"
import { createApi } from "./api"
const app = new Hono()
app.route("/api", createApi(createLibrary(resolve(process.env.CONTEXTUAL_DATABASE ?? "data/diagrams.sqlite"))))
app.all("/api/*", c => c.notFound())
app.use("/assets/*", serveStatic({ root: "./dist-web" }))
app.get("*", serveStatic({ path: "./dist-web/index.html" }))
serve({ fetch: app.fetch, hostname: "127.0.0.1", port: Number(process.env.PORT ?? 3000) })
