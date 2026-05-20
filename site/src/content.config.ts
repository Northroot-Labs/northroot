import { defineCollection } from "astro:content";
import { glob } from "astro/loaders";
import { z } from "astro/zod";

const sourceRef = z.object({
  repo: z.string(),
  path: z.string().optional(),
  commit: z.string().optional()
});

const docs = defineCollection({
  loader: glob({ base: "./src/content/docs", pattern: "**/*.{md,mdx}" }),
  schema: z.object({
    title: z.string(),
    description: z.string(),
    section: z.enum(["product", "systems", "examples", "architecture", "docs", "journal"]),
    sourceRefs: z.array(sourceRef).default([]),
    updated: z.coerce.date().optional()
  })
});

const fieldNotes = defineCollection({
  loader: glob({ base: "./src/content/field-notes", pattern: "**/*.{md,mdx}" }),
  schema: z.object({
    title: z.string(),
    summary: z.string(),
    date: z.coerce.date(),
    draft: z.boolean().default(false),
    sourceRefs: z.array(sourceRef).default([])
  })
});

const candidates = defineCollection({
  loader: glob({ base: "./src/content/candidates", pattern: "**/*.{md,mdx}" }),
  schema: z.object({
    title: z.string(),
    summary: z.string(),
    date: z.coerce.date(),
    draft: z.boolean().default(true),
    generatedBy: z.string(),
    sourceRefs: z.array(sourceRef).default([])
  })
});

export const collections = { candidates, docs, fieldNotes };
