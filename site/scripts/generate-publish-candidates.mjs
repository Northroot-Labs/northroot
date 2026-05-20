#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import process from "node:process";

const args = new Map();
for (let index = 2; index < process.argv.length; index += 1) {
  const arg = process.argv[index];
  if (arg.startsWith("--")) {
    const next = process.argv[index + 1];
    if (!next || next.startsWith("--")) {
      args.set(arg, true);
    } else {
      args.set(arg, next);
      index += 1;
    }
  }
}

const source = path.resolve(String(args.get("--source") ?? "../northroot"));
const outDir = path.resolve(String(args.get("--out") ?? "src/content/candidates"));
const since = String(args.get("--since") ?? "7 days ago");
const dryRun = args.has("--dry-run");
const maxCommits = Number(args.get("--max-commits") ?? 30);

const allowedPathPrefixes = [
  "README.md",
  "GOVERNANCE.md",
  "CORE_INVARIANTS.md",
  "DOMAIN_INVARIANTS.md",
  "docs/",
  "examples/",
  "fixtures/",
  "schemas/"
];

function git(args) {
  return execFileSync("git", ["-C", source, ...args], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"]
  }).trim();
}

function isAllowedSourcePath(filePath) {
  return allowedPathPrefixes.some((prefix) => filePath === prefix || filePath.startsWith(prefix));
}

function yamlString(value) {
  return JSON.stringify(value);
}

function slugForDate(date) {
  return `${date.toISOString().slice(0, 10)}-northroot-publish-candidates`;
}

if (!existsSync(source)) {
  console.error(`Source repo does not exist: ${source}`);
  process.exit(1);
}

const root = git(["rev-parse", "--show-toplevel"]);
const repoName = path.basename(root);
if (repoName !== "northroot") {
  console.error(`Refusing to scan non-northroot source repo: ${root}`);
  process.exit(1);
}

const logOutput = git([
  "log",
  `--since=${since}`,
  `--max-count=${maxCommits}`,
  "--date=short",
  "--pretty=format:%H%x09%ad%x09%s",
  "--",
  ...allowedPathPrefixes
]);

if (!logOutput) {
  console.log(`No northroot publish-candidate changes found since ${since}.`);
  process.exit(0);
}

const commits = logOutput.split("\n").map((line) => {
  const [hash, date, ...subjectParts] = line.split("\t");
  return { hash, date, subject: subjectParts.join("\t") };
});

const changedFiles = new Map();
for (const commit of commits) {
  const files = git(["diff-tree", "--no-commit-id", "--name-only", "-r", commit.hash])
    .split("\n")
    .filter(Boolean)
    .filter(isAllowedSourcePath);
  for (const file of files) {
    if (!changedFiles.has(file)) {
      changedFiles.set(file, commit.hash);
    }
  }
}

const now = new Date();
const slug = slugForDate(now);
const target = path.join(outDir, `${slug}.mdx`);
const sourceRefs = [...changedFiles.entries()].slice(0, 16);

const frontmatter = [
  "---",
  `title: ${yamlString(`Northroot publication drafts for ${now.toISOString().slice(0, 10)}`)}`,
  `summary: ${yamlString("Generated draft material from recent public Northroot docs, examples, and spec changes.")}`,
  `date: ${now.toISOString().slice(0, 10)}`,
  "draft: true",
  `generatedBy: ${yamlString("scripts/generate-publish-candidates.mjs")}`,
  "sourceRefs:",
  ...sourceRefs.map(([file, commit]) => `  - repo: northroot\n    path: ${yamlString(file)}\n    commit: ${yamlString(commit.slice(0, 12))}`),
  "---"
].join("\n");

const body = [
  "",
  "This generated draft is an editorial input. Edit it before publishing.",
  "",
  "## Possible Angles",
  "",
  "- Turn a changed spec into a short public explanation.",
  "- Turn a changed example into a runnable walkthrough.",
  "- Summarize notable kernel work without exposing private context.",
  "",
  "## Recent Public Source Commits",
  "",
  ...commits.map((commit) => `- \`${commit.hash.slice(0, 12)}\` ${commit.date} - ${commit.subject}`),
  "",
  "## Changed Public Source Paths",
  "",
  ...[...changedFiles.keys()].sort().map((file) => `- \`northroot/${file}\``),
  "",
  "## Editorial Checklist",
  "",
  "- Confirm every source reference is public.",
  "- Remove commit noise that does not support a clear public point.",
  "- Prefer one focused post over a broad changelog.",
  "- Keep canonical spec language in `northroot`; use the site for explanation.",
  ""
].join("\n");

const content = `${frontmatter}\n${body}`;

if (dryRun) {
  console.log(content);
  process.exit(0);
}

mkdirSync(outDir, { recursive: true });
writeFileSync(target, content, "utf8");
console.log(`Wrote ${path.relative(process.cwd(), target)}`);
