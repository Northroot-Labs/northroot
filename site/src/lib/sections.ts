export const navSections = [
  {
    id: "product",
    label: "Product",
    summary: "Portable evidence, kernel scope, current primitives, and non-goals."
  },
  {
    id: "systems",
    label: "Systems",
    summary: "Publishing boundaries, source authority, and review flow."
  },
  {
    id: "docs",
    label: "Docs",
    summary: "Reading paths into public specs and source-owned references."
  },
  {
    id: "journal",
    label: "Journal",
    summary: "Field notes and format decisions from implementation work."
  },
  {
    id: "examples",
    label: "Examples",
    summary: "Runnable evidence bundles for events, receipts, hashes, and offline verification."
  }
] as const;

export const auxiliarySections = [
  {
    id: "architecture",
    label: "Architecture",
    summary: "Design notes for the trust kernel and the runtimes that compose around it."
  },
  {
    id: "field-notes",
    label: "Field Notes",
    summary: "Engineering notes from real implementation work."
  }
] as const;

export const allSections = [...navSections, ...auxiliarySections] as const;

export const contentSections = [...navSections, auxiliarySections[0]] as const;

export type ContentSectionId = (typeof contentSections)[number]["id"];

export function sectionById(id: string) {
  return allSections.find((section) => section.id === id);
}
