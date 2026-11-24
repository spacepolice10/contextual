import test from "node:test"
import assert from "node:assert/strict"
import { focusedFlow } from "../src/renderer/focused-view.ts"

const nodes = [
  { id: 1, parentId: null, kind: "root", name: "Root", contentIds: [] },
  { id: 2, parentId: 1, kind: "goal", name: "Goal", contentIds: [20] },
  { id: 3, parentId: 2, kind: "task", name: "Task", contentIds: [30] },
  { id: 4, parentId: 1, kind: "goal", name: "Sibling", contentIds: [40] },
  { id: 5, parentId: 3, kind: "evidence", name: "Evidence", contentIds: [50] },
]
const flow = { nodes, contents: [20, 30, 40, 50].map(id => ({ id })) }
test("projects the subtree, detaches root, preserves IDs and source", () => {
  const result = focusedFlow(flow, 2)
  assert.deepEqual(result.nodes.map(n => n.id), [2, 3, 5])
  assert.equal(result.nodes[0].parentId, null)
  assert.equal(result.nodes[0].kind, "goal")
  assert.deepEqual(result.contents.map(c => c.id), [20, 30, 50])
  assert.equal(flow.nodes[1].parentId, 1)
})
test("supports a single terminal node and nested focus", () => {
  assert.deepEqual(focusedFlow(flow, 5).nodes.map(n => n.id), [5])
  assert.deepEqual(focusedFlow(focusedFlow(flow, 2), 3), focusedFlow(flow, 3))
})
test("rejects missing or invalid IDs", () => {
  for (const id of [0, -1, NaN, 2.5, 999]) assert.equal(focusedFlow(flow, id), undefined)
})
test("terminates on a cyclic hierarchy", () => {
  const cyclic = { ...flow, nodes: nodes.map(n => n.id === 2 ? { ...n, parentId: 5 } : n) }
  assert.deepEqual(focusedFlow(cyclic, 2).nodes.map(n => n.id), [2, 3, 5])
})
