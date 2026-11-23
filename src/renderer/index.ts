import { getPagePath, redirectPage } from "@nanostores/router"
import { atom, computed } from "nanostores"
import { define } from "nanotags"
import { renderList } from "nanotags/render"
import { mount } from "pele"
import type { Diagram, DiagramLibrary } from "../diagrams"
import { $router } from "./router"
import "./styles.css"

declare global {
  interface Window {
    diagrams: DiagramLibrary
  }

  interface HTMLElementTagNameMap {
    "contextual-library": InstanceType<typeof ContextualLibrary>
  }
}

type LibraryState =
  | { status: "loading" }
  | { status: "ready"; diagrams: Diagram[] }
  | { status: "error" }

const ContextualLibrary = define("contextual-library")
  .withRefs((r) => ({
    list: r.one("ul"),
    item: r.one("template"),
    title: r.one("h1"),
    canvas: r.one("section"),
    message: r.one("p"),
    diagram: r.one("div"),
  }))
  .setup((ctx) => {
    const { list, item, title, canvas, message, diagram } = ctx.refs
    const $library = atom<LibraryState>({ status: "loading" })
    const $selected = computed([$library, $router], (state, page) => {
      if (state.status === "ready" && page?.route === "diagram") {
        return state.diagrams.find(({ id }) => String(id) === page.params.id)
      }
    })

    async function loadLibrary(signal: AbortSignal) {
      try {
        const diagrams = await window.diagrams.list()
        if (signal.aborted) return

        const first = diagrams[0]
        if (first && $router.get()?.route === "library") {
          redirectPage($router, "diagram", { id: String(first.id) })
        }
        $library.set({ status: "ready", diagrams })
      } catch (error) {
        console.error(error)
        if (!signal.aborted) $library.set({ status: "error" })
      }
    }

    ctx.effect([$library, $router, $selected], (state, page, selected) => {
      renderList(list, item, {
        data: state.status === "ready" ? state.diagrams.map(({ id, title }) => ({
          id,
          title,
          current: id === selected?.id,
        })) : [],
        key: ({ id }) => id,
        update: (row, { id, title, current }) => {
          const link = ctx.getElement(row, "a")
          link.textContent = title
          link.href = getPagePath($router, "diagram", { id: String(id) })
          if (current) link.setAttribute("aria-current", "page")
          else link.removeAttribute("aria-current")
        },
      })

      canvas.setAttribute("aria-busy", String(state.status === "loading"))
      if (state.status === "loading") message.textContent = "Loading diagrams…"
      else if (state.status === "error") message.textContent = "The diagram library could not be opened."
      else if (!page) message.textContent = "This page could not be found."
      else if (page.route === "diagram" && !selected) message.textContent = "This diagram could not be found."
      else if (state.diagrams.length === 0) message.textContent = "No diagrams in this library."
      else message.textContent = ""
      message.hidden = message.textContent === ""
    })

    ctx.effect($selected, (selected) => {
      title.textContent = selected?.title ?? "Diagram library"
      diagram.hidden = !selected
      if (!selected) return

      const preview = mount(diagram, selected.source, {
        links: false,
        images: false,
        onRender: () => {
          const svg = diagram.querySelector("svg")
          svg?.setAttribute("role", "img")
          svg?.setAttribute("aria-label", selected.title)
        },
      })

      return () => {
        preview.destroy()
        diagram.replaceChildren()
      }
    })

    const request = new AbortController()
    ctx.onCleanup(() => request.abort())
    void loadLibrary(request.signal)
  })
