import { atom, computed } from "nanostores"
import { define } from "nanotags"
import { renderList } from "nanotags/render"
import { mount } from "pele"
import type { Diagram, DiagramLibrary } from "../diagrams"
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
  | { status: "ready"; diagrams: Diagram[]; selectedId: number | undefined }
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
    const $selected = computed($library, (state) => {
      if (state.status === "ready") return state.diagrams.find(({ id }) => id === state.selectedId)
    })

    async function loadLibrary(signal: AbortSignal) {
      try {
        const diagrams = await window.diagrams.list()
        if (!signal.aborted) {
          $library.set({ status: "ready", diagrams, selectedId: diagrams[0]?.id })
        }
      } catch (error) {
        console.error(error)
        if (!signal.aborted) $library.set({ status: "error" })
      }
    }

    ctx.on(list, "click", (event) => {
      if (!(event.target instanceof Element)) return
      const button = event.target.closest<HTMLButtonElement>("button[data-diagram-id]")
      const state = $library.get()
      const id = Number(button?.dataset.diagramId)
      if (state.status === "ready" && state.diagrams.some((diagram) => diagram.id === id)) {
        $library.set({ ...state, selectedId: id })
      }
    })

    ctx.effect($library, (state) => {
      const ready = state.status === "ready"
      renderList(list, item, {
        data: ready ? state.diagrams.map(({ id, title }) => ({
          id,
          title,
          current: id === state.selectedId,
        })) : [],
        key: ({ id }) => id,
        update: (row, { id, title, current }) => {
          const button = ctx.getElement(row, "button")
          button.textContent = title
          button.setAttribute("data-diagram-id", String(id))
          if (current) button.setAttribute("aria-current", "true")
          else button.removeAttribute("aria-current")
        },
      })

      canvas.setAttribute("aria-busy", String(state.status === "loading"))
      if (state.status === "loading") message.textContent = "Loading diagrams…"
      else if (state.status === "error") message.textContent = "The diagram library could not be opened."
      else message.textContent = ready && state.diagrams.length === 0 ? "No diagrams in this library." : ""
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
