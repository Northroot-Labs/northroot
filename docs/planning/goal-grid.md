# Goal Grid (Context Only)

Northroot uses a goal grid as a planning framework. It provides a fixed structure for breaking a single high-level goal into 8 supporting pillars, each with 8 concrete tasks (64 total). This gives us:

- a canonical scope boundary,
- stable task identifiers (P*-T*),
- a deterministic plan agents can align to,
- and a disciplined way to track progress across versions.

This is not a motivational system. It is a simple hierarchical grid that forces clarity and prevents drift. The active goal grid file defines the only valid tasks for the current version of Northroot.

## Structure

- **Central Goal (CG-1)**: The primary objective
- **8 Pillars (P1-P8)**: Key areas essential for achieving the goal
- **8 Tasks per Pillar (P*-T1 through P*-T8)**: Specific, actionable subtasks
- **Total: 64 cells** (8 pillars × 8 tasks)

## Task Identification

Every task has a canonical identifier:
- Format: `P*-T*` (e.g., `P1-T1`, `P2-T8`)
- **P*** = Pillar number (1-8)
- **T*** = Task number (1-8)

This allows:
- Clear progress tracking
- Commit message references
- ADR task mapping
- Scope drift prevention

## Active Plan

The active goal grid plan is maintained in:
- **Source of Truth**: [`goals/harada/northroot-active.md`](../../goals/harada/northroot-active.md)
- **Progress Tracking**: [`docs/adr/ADR-0012-harada-v01-alignment/HARADA_PROGRESS.md`](../adr/ADR-0012-harada-v01-alignment/HARADA_PROGRESS.md)
- **ADR Reference**: [`docs/adr/ADR-0012-harada-v01-alignment/ADR-0012.md`](../adr/ADR-0012-harada-v01-alignment/ADR-0012.md)

## Benefits for Software Development

1. **Scope Control**: The fixed 64-cell structure prevents feature creep
2. **Clear Prioritization**: Pillars and tasks create a natural hierarchy
3. **Progress Visibility**: Easy to see what's complete, in progress, or pending
4. **Task Traceability**: Every commit, ADR, and decision can reference a task ID
5. **Discipline**: Enforces focus on the plan rather than ad-hoc additions

## Background

This planning framework is inspired by the Harada Method, a goal-setting system developed by Takashi Harada. Northroot adapted the structural grid approach (64-cell organization) for v0.1 planning to maintain focus and prevent scope drift.

---

**Note**: The goal grid is a planning framework, not a software development methodology. We use it for goal-setting and task organization, while following standard software engineering practices (versioning, testing, documentation, etc.) for implementation.

