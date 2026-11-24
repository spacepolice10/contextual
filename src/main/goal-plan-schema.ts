import { z } from "zod"

export const goalPlanSchema = z.strictObject({
  groups: z.array(z.strictObject({
    mode: z.enum(["sequential", "parallel"]).default("sequential"),
    tasks: z.array(z.string().trim().min(1).max(400)).min(1).max(12),
  })).min(1).max(12),
}).refine(({ groups }) => {
  const tasks = groups.flatMap((group) => group.tasks)
  return tasks.length <= 12 && new Set(tasks).size === tasks.length
}, "Use at most 12 distinct tasks across the plan.")

const name = z.string().trim().min(1).max(220)
const body = z.string().trim().min(1).max(16000)
const attachment = z.strictObject({ name, body: body.optional() })

export const taskResultSchema = z.strictObject({
  solution: z.strictObject({ name, body }),
  comments: z.array(attachment).max(6),
  evidence: z.array(attachment).max(6),
})
