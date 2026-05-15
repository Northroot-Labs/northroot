# `.nrj` Format Decision

Status: optional export container

Northroot starts with JSON and JSONL because they are inspectable with ordinary
tools, easy to diff, and enough for the first obligation, policy, event, receipt,
and cost attribution examples.

The custom `.nrj` format is retained only where it provides concrete value:

- binary framing without delimiter ambiguity
- explicit record kinds with skip-unknown behavior
- streaming verification over large audit exports
- strict and permissive truncation handling
- cross-language offline verifier stability

If those properties are not needed for a workflow, use JSON/JSONL plus canonical
hashes. If future `.nrj` work cannot keep those properties covered by tests and
public examples, archive the format as historical design rather than making it a
default substrate requirement.
