import { z } from "zod"

export const goalPlanSchema = z.strictObject({
  tasks: z.array(z.string().trim().min(1).max(400)).min(1).max(12)
    .refine((tasks) => new Set(tasks).size === tasks.length, "Tasks must be distinct."),
})

const name = z.string().trim().min(1).max(220)
const body = z.string().trim().min(1).max(16000)
const attachment = z.strictObject({ name, body: body.optional() })

export const taskResultSchema = z.strictObject({
  solution: z.strictObject({ name, body }),
  comments: z.array(attachment).max(6),
  evidence: z.array(attachment).max(6),
})
