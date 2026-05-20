# Northroot Site

Markdown-first public site for Northroot product notes, systems docs,
source-backed examples, journal entries, and draft publication material.

This site lives inside the public `northroot` repository. Canonical source
material remains in the source paths that implement and test it.

## Public site

- `Product` explains the kernel boundary and current non-goals.
- `Systems` explains publishing boundaries and source ownership.
- `Docs` provides reading paths into source-owned references.
- `Journal` captures implementation notes and format decisions.
- `Examples` walks through runnable public evidence bundles.

Internal tools, operating consoles, sign-in surfaces, and access-control product
claims are intentionally out of scope for this public site.

## Local editing

```bash
npx -y pnpm@11.1.3 --dir site install
npx -y pnpm@11.1.3 --dir site dev
```

Open `http://localhost:4321` to preview local edits.

## Checks

```bash
npx -y pnpm@11.1.3 --dir site check
npx -y pnpm@11.1.3 --dir site build
npx -y pnpm@11.1.3 --dir site candidate:dry-run
```

## Draft generation

The scheduled job scans public `northroot` source paths only. It generates draft
Markdown material for human review and opens a draft pull request. It does not
publish, deploy, merge, or mutate canonical source docs.
