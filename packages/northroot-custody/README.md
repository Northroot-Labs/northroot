# northroot-custody

`northroot-custody` is the public-safe Northroot package for custody contracts,
delegated snapshot plans, and the steward profile helper layer.

The reusable custody vocabulary is intentionally small: workspace inventory,
policy, snapshot plan, verification result, retention decision, and run summary.

Workspace inventories may also include object-level custody state. This is the
layer that makes stewardship more precise than Git visibility:

- `object_id`: stable symbolic identifier;
- `object_type`: `repo`, `sqlite`, `postgres`, `secret-file`, `env-file`,
  `artifact-dir`, `journal`, `cache`, or `generated-state`;
- `visibility`: `public`, `private`, `secret`, `regulated`, or `ephemeral`;
- `storage_binding`: symbolic binding such as `workspace://...`,
  `repository://...`, `secret://...`, `env://...`, or `provider://...`;
- `custody_policy`: object-level backup, retention, verification, and restore
  obligations;
- `redaction_policy`: what may appear in public summaries and agent-facing
  reports;
- `restore_class`: `full-restore`, `metadata-only`,
  `rehydrate-from-provider`, or `never-export`.

Real paths, provider references, external volumes, LaunchAgent labels, logs,
and backup receipt locations stay in private binding documents or deployment
state. Public inventories use symbolic bindings only.

The steward service registry is the next layer above a single inventory and
policy. A `northroot.steward.service-registry.v0` document registers the node,
projects, object-level and project-level permission sets, destinations,
source-to-destination bindings, replicas, legacy import refs, and fail-closed
resume policy. `steward registry` stores that document as durable state with
atomic writes, a single operation lock, operation summaries, and
recover-before-next-mutation behavior. It is still a contract and verification
surface, not a storage transport or scheduler.

Registry failure handling should assume boring operational failures:
disconnected storage, interrupted runs, power loss, stale generated artifacts,
and partial delegated operations. The public contract requires symbolic refs,
single-flight/lock strategy metadata, explicit resume behavior, and a
`never-prune-without-retention-decision` style partial-run policy before
replicas or legacy imports are considered safe.

It is not a backup engine, scheduler, secret manager, storage transport, or
monitoring stack. Execution is delegated to established tools such as `restic`,
`resticprofile`, `launchd`, `systemd`, 1Password service-account based secret
resolution, `rclone`, external health monitors, and database-native backup
tools.

```python
from pathlib import Path
from northroot.custody import init_steward, render_snapshot_plan
```

Install locally from the Northroot repository while this package is still
workspace-local:

```bash
python -m pip install -e packages/northroot-custody
nr-custody validate packages/northroot-custody/examples/workspace-inventory.example.json --public-safe
```

```bash
nr-custody validate examples/workspace-inventory.example.json --public-safe
nr-custody validate examples/custody-policy.example.json --public-safe
nr-custody validate examples/snapshot-plan.example.json --public-safe
nr-custody validate examples/verification-result.example.json --public-safe
nr-custody validate examples/retention-decision.example.json --public-safe
nr-custody validate examples/run-summary.example.json --public-safe
nr-custody validate examples/command-plan.example.json --public-safe
nr-custody validate examples/service-registry.example.json --public-safe
nr-custody validate examples/legacy-profile-import.redacted.example.json --public-safe
nr-custody validate examples/legacy-run-import.redacted.example.json --public-safe
nr-custody validate examples/agent-delegation-policy.dogfood.example.json --public-safe
nr-custody validate examples/secret-bindings.macos-keychain.example.json --public-safe
nr-custody validate examples/repository-bindings.redacted.example.json --public-safe
nr-custody render-plan \
  --inventory examples/workspace-inventory.example.json \
  --policy examples/custody-policy.example.json
nr-custody evaluate-retention \
  --policy examples/custody-policy.example.json \
  --snapshot-id snap-001 \
  --evidence verified_snapshot \
  --evidence verified_offsite_copy \
  --evidence restore_drill
nr-custody steward init \
  --inventory examples/workspace-inventory.example.json \
  --policy examples/custody-policy.example.json \
  --secret-bindings examples/secret-bindings.redacted.example.json \
  --repository-bindings examples/repository-bindings.redacted.example.json \
  --output /tmp/northroot-steward-example
nr-custody steward status --state /tmp/northroot-steward-example
nr-custody steward capabilities --state /tmp/northroot-steward-example
nr-custody steward command-plan \
  --state /tmp/northroot-steward-example \
  --operation restore \
  --snapshot-id snap-001 \
  --target /tmp/northroot-recovery-restore
nr-custody steward preflight --state /tmp/northroot-steward-example
nr-custody steward report --state /tmp/northroot-steward-example --snapshot-id snap-001
nr-custody steward verify-state --state /tmp/northroot-steward-example --snapshot-id snap-001
nr-custody steward verify-state \
  --state /tmp/northroot-steward-example \
  --registry-state /tmp/northroot-steward-registry \
  --project-id project/example
nr-custody steward report \
  --state /tmp/northroot-steward-example \
  --registry-state /tmp/northroot-steward-registry \
  --project-id project/example
nr-custody steward run --state /tmp/northroot-steward-example --snapshot-id snap-001
nr-custody steward run \
  --state /tmp/northroot-steward-example \
  --registry-state /tmp/northroot-steward-registry \
  --project-id project/example \
  --snapshot-id snap-001 \
  --execute
nr-custody steward import-legacy-runs \
  --state /tmp/northroot-steward-example \
  --json examples/legacy-run-import.redacted.example.json \
  --public-safe
nr-custody steward recover-operation --state /tmp/northroot-steward-example
nr-custody steward verify --state /tmp/northroot-steward-example --snapshot-id snap-001
nr-custody steward restore \
  --state /tmp/northroot-steward-example \
  --snapshot-id snap-001 \
  --target /tmp/northroot-recovery-restore
nr-custody steward restore-drill --state /tmp/northroot-steward-example --snapshot-id snap-001
nr-custody steward offsite report --state /tmp/northroot-steward-example --snapshot-id snap-001
nr-custody steward evidence record \
  --state /tmp/northroot-steward-example \
  --snapshot-id snap-001 \
  --evidence verified_offsite_copy \
  --source external-monitor://offsite-copy-check \
  --artifact-ref artifact://private/offsite-check/run-001
nr-custody steward evidence report --state /tmp/northroot-steward-example --snapshot-id snap-001
nr-custody steward retention evaluate \
  --state /tmp/northroot-steward-example \
  --snapshot-id snap-001 \
  --use-recorded-evidence
nr-custody steward registry init \
  --state /tmp/northroot-steward-registry \
  --registry examples/service-registry.example.json \
  --public-safe
nr-custody steward registry status --state /tmp/northroot-steward-registry --public-safe
nr-custody steward registry verify --state /tmp/northroot-steward-registry --public-safe
nr-custody steward registry topology \
  --state /tmp/northroot-steward-registry \
  --project-id project/example \
  --public-safe
nr-custody steward registry authorize \
  --state /tmp/northroot-steward-registry \
  --operation run \
  --project-id project/example \
  --public-safe
nr-custody steward registry add-object \
  --state /tmp/northroot-steward-registry \
  --json /tmp/object-custody.json \
  --public-safe
nr-custody steward registry register-project \
  --state /tmp/northroot-steward-registry \
  --project-json /tmp/project.json \
  --permission-json /tmp/project-permission.json \
  --public-safe
nr-custody steward registry import-legacy-profile \
  --state /tmp/northroot-steward-registry \
  --json examples/legacy-profile-import.redacted.example.json \
  --public-safe
nr-custody steward registry recover --state /tmp/northroot-steward-registry --public-safe
nr-custody steward schedule create \
  --state /tmp/northroot-steward-example \
  --scheduler launchd \
  --operation run \
  --every-minutes 60 \
  --registry-state /tmp/northroot-steward-registry \
  --project-id project/example
nr-custody steward schedule status --state /tmp/northroot-steward-example
nr-custody steward schedule install --state /tmp/northroot-steward-example
nr-custody steward schedule install --state /tmp/northroot-steward-example --execute
nr-custody steward schedule install --state /tmp/northroot-steward-example --execute --skip-preflight
nr-custody steward schedule uninstall --state /tmp/northroot-steward-example --execute
nr-custody steward schedule delete --state /tmp/northroot-steward-example
nr-custody steward schedule delete --state /tmp/northroot-steward-example --force
```

`steward run`, `steward verify`, `steward restore`, and
`steward restore-drill` render delegated `resticprofile` commands and record
run summaries by default. Pass `--execute` only when the local machine has the
required commodity tooling and private secret bindings configured. Pass
`--snapshot-id` when the resulting evidence is intended to satisfy retention
gates for a specific snapshot.

`steward preflight` is the readiness gate for unattended operation. It checks
the generated steward files, `resticprofile` availability, private repository
binding validity, private secret binding validity, provider command
availability, and required scheduler environment variables or runtime
environment bindings without printing secret values. Steward records hashes for
generated custody artifacts such as `snapshot-plan.json` and
`resticprofile.yaml`; preflight fails if those generated files drift. Change the
inventory, policy, or private bindings and rerun `steward init` instead of
hand-editing generated config. The root `steward-installation.json` manifest is
also indexed in `steward-installation-index.json` with
`northroot.steward.installation-index.v0`; preflight fails closed if that root
manifest is unindexed, missing, or digest-mismatched because it controls the
paths and generated artifact hashes used by the rest of steward state.

`steward verify-state` is the read-only aggregate gate for agents and daemons.
It composes status, preflight, the capability manifest, schedule metadata, and
snapshot-scoped evidence into a `northroot.steward.state-verification.v0`
document. With `--snapshot-id`, it also evaluates whether recorded evidence is
sufficient for that snapshot's retention gate. It does not execute backups,
install schedules, run restore drills, or record evidence.

Pass `--registry-state` to include service-registry integrity in the aggregate
state check. Pass `--project-id` and optional `--object-id` to also prove the
registry authorizes `verify-state` for that project/object context. A tampered,
unindexed, locked, or unauthorized registry makes the aggregate check fail
closed.

Steward run summaries are indexed in `run-summaries/index.json` with
`northroot.steward.run-summary-index.v0` digest entries. Evidence reports only
trust summaries that match this index, and `steward verify-state` fails closed
with `run_summary_integrity_failed` when summaries are missing from the index,
missing from disk, or digest-mismatched. This keeps retention evidence tied to
steward-recorded state instead of loose JSON files.

`steward report` is the read-only operator and agent report. It composes status,
preflight, schedule state, evidence, offsite copy status, retention readiness,
and recommended next actions into `northroot.steward.report.v0`. It is not a
gate and does not execute backups, install schedules, run restore drills, or
record evidence. It accepts the same registry context flags as `verify-state`
and includes registry readiness, protected-state status, authorization decision,
and repair guidance when the policy proof is not usable.

`steward registry` is the durable state-management surface for the service
registry. `registry init` validates and installs a public-safe registry
document. Mutation commands append object custody entries, project/object
permissions, registered projects, destinations, source bindings, replicas, and
legacy import records through atomic writes. Each successful mutation records a
registry operation summary with the resulting registry digest. Registry
operation summaries are also indexed in `registry-operations/index.json` with
`northroot.steward.registry-operation-index.v0` digest entries. `registry verify`
checks both the live registry digest and the operation-log digest index,
so a structurally valid but unrecorded registry edit, or a hand-edited operation
summary, is not treated as protected state. `registry status`, `registry
authorize`, and mutation commands depend on that proof before treating the
registry as ready. Unreadable or corrupted registry JSON is reported as
structured not-ready state and authorization returns `invalid-registry` instead
of allowing automation to proceed. Registry initialization and later mutations
both run under registry operation locks, so a machine death or interrupted
process fails closed until `registry recover` records the interrupted operation.
Operation locks include the registry digest observed before the write when one
exists; recovery records `resume_state` as
`registry-unchanged-after-lock`, `registry-changed-after-lock`,
`registry-change-unknown`, or `registry-missing-after-lock` so operators can tell
whether the interrupted write appears to have landed. Recovery removes the lock
only when the registry still validates, or when initialization stopped before
the registry file was written and can be retried cleanly.

`steward registry bind-source` attaches a source destination to the referenced
project as part of the same atomic registry mutation. Callers do not need to
hand-edit `project.source_destination_ids` after adding a source binding.

`steward registry topology` is a read-only readiness report for project
destination wiring. It expands registered projects into source destinations,
destination bindings, replica targets, required replica evidence, object
visibility/restore classes, and the fail-closed resume policy. It returns
non-zero when the registry is locked or invalid, when the requested project is
unknown, when a project has no usable source destination, or when replica
readiness lacks required evidence or the registry-level
`on_disconnected_storage` / `partial_run_handling` policy is not fail-closed.
It also checks that each source binding belongs to the project that references
it, uses a source-compatible destination role, uses the project permission set,
and only includes objects in that project; replica targets must use a replica
destination role and require `verified_offsite_copy` evidence.
It does not probe storage or copy bytes; private deployments still use
repository binding availability probes and external copy monitors for live
storage availability.

`steward registry import-legacy-profile` applies a sanitized legacy migration
bundle such as `examples/legacy-profile-import.redacted.example.json` as one
atomic registry mutation. The bundle must contain symbolic refs and redacted
object custody entries, not raw LaunchAgent paths, machine-local state paths,
volume names, secret values, or private receipts. Replaying an identical import
skips existing entries; conflicting entries fail closed without changing the
registry.

`steward import-legacy-runs` imports sanitized historical run summaries from a
legacy profile into the steward state's `run-summaries/` directory. It accepts
`northroot.steward.legacy-run-import.v0` bundles, writes each run summary
atomically, skips identical replays, rejects conflicting `run_id`s, and uses the
steward operation lock so interrupted imports require `steward recover-operation`
before retry. Imported summaries are added to the steward run-summary digest
index and feed the same `evidence report` and retention checks as native steward
runs.

`steward registry authorize` is the deterministic permission gate for agents and
automation. It evaluates a project operation, and optionally an object-scoped
operation, against project and object permission sets. Blocked operations,
human-clearance operations, missing allow rules, invalid registries, and
unresolved recovery locks all return non-zero.

`examples/agent-delegation-policy.dogfood.example.json` is the default dogfood
delegation policy for current steward work. It registers `agent:codex` for
`codex/` branches and allows branch checkout/creation, checkpoint commits,
branch pushes, draft PR open/update, PR follow-up, and verification under
explicit agent identity metadata. It still prohibits protected-branch pushes,
protected-branch merges, workflow permission escalation, long-lived signing key
access, and human-author impersonation.

`steward capabilities` publishes that same default under `agent_contract`, and
`steward command-plan` can plan those dogfood operations without a separate
policy argument. For example, a registered Codex agent can request
`--operation branch.create --branch codex/<topic>`, `--operation commit.create`
with a commit message and verification summary, `--operation push.branch`, or
draft PR operations. The returned plan includes required agent provenance
trailers and still refuses protected branches or unregistered agents. Draft PRs
remain draft until final human review clearance.

Delegated `steward run`, `steward verify`, `steward restore`, and
`steward restore-drill` accept `--registry-state`, `--project-id`, and optional
`--object-id`. When those arguments are supplied with `--execute`, registry
authorization runs before preflight or delegated tooling. Denied operations
write a run summary with `delegated-authorization-denied` and do not call the
external backup or restore command.

Executed delegated operations are guarded by `steward-operation.lock.json` in
the steward state directory. A stale lock blocks later execution with
`delegated-operation-locked`; run `steward recover-operation --state ...` to
record `delegated-interrupted-recovered` and clear the lock before retrying.
Unreadable or partially written operation locks also fail closed; recovery
records `delegated-invalid-lock-recovered` with the lock digest/error before
clearing the lock.
`steward verify-state`, `steward report`, and `steward command-plan` also fail
closed around that lock so automation cannot treat interrupted steward state as
safe to resume blindly.

`steward restore` is the bounded recovery path for an actual restore. It
requires both `--snapshot-id` and `--target`, delegates to `resticprofile`, and
records an observed restore target with file count, byte count, and manifest
hash when executed. It is intentionally not schedulable and does not satisfy
the `restore_drill` retention evidence; keep restore drills as separate,
repeatable readiness checks.

Scheduled templates call the selected operation with `--execute`. `--operation`
can be `run`, `verify`, or `restore-drill`. `schedule create` only renders
templates; `schedule install --execute` delegates installation to `launchctl` or
`systemctl --user` after preflight passes; `schedule uninstall --execute`
removes the platform registration; `schedule delete` removes generated template
files only when the schedule is not marked installed. `schedule create`,
`schedule status`, `schedule install`, `schedule uninstall`, and
`schedule delete` accept `--registry-state`, `--project-id`, and optional
`--object-id`; when present, registry authorization runs before schedule state
or platform registration is changed. Registry context supplied at schedule
creation is persisted in `schedule.json`, used by later schedule subcommands,
and rendered into the unattended scheduled command so hourly runs keep the same
project/object policy context. Schedules created without registry context keep
the profile-level `schedules/schedule.json` path for compatibility. Schedules
created with registry/project/object context are written under
`schedules/contexts/<schedule-scope-id>/` with distinct launchd labels or
systemd unit names, so multiple registered projects can keep separate schedules
without overwriting each other. When exactly one scoped schedule exists,
context-free status/install/uninstall/delete commands use it for compatibility;
when multiple scoped schedules exist, those commands fail closed until the
registry/project/object context is supplied. Use `schedule delete --force` only for
operator-confirmed cleanup of stale generated files after the platform
registration has already been removed. Refused deletion returns a non-zero exit
status so scripts and agents cannot mistake a blocked cleanup for success.
`--skip-preflight` is available for explicit operator-controlled exceptions.
Executed operations fail closed when preflight is not ready and still write an
auditable run summary with `delegated-preflight-failed` status. Scheduled
operations do not invent a snapshot id; treat their evidence as general custody
health unless the generated command is deliberately bound to a specific
snapshot.

The generated schedule records its `runner_command` and preflight checks that
the command is executable in the current environment. For unattended launchd or
systemd jobs, prefer an absolute runner path or a deliberately installed `nr`
shim instead of relying on an interactive shell PATH. Schedule rendering parses
the runner command with shell rules, appends steward arguments, shell-quotes
paths, and escapes launchd XML so state directories with spaces or XML-sensitive
characters do not break unattended execution. If the runner executable path
contains spaces, quote it inside `--runner-command`, for example
`--runner-command "'/opt/Northroot Tools/nr' steward"`. Generated schedule
templates are written with fsync-and-rename semantics and hashed in
`schedule.json`; preflight fails if a rendered launchd plist or systemd
unit/timer drifts after creation. The `schedule.json` manifest itself is
indexed in its sibling `schedule-index.json` with
`northroot.steward.schedule-index.v0`, and schedule install, uninstall, delete,
and preflight fail closed if that manifest is unindexed, missing, or
digest-mismatched. If generated scheduler files exist without a schedule
manifest, steward reports `orphaned-artifacts` and requires
`schedule delete --force` after the platform registration state has been
confirmed. Use `schedule delete --force` only for explicit cleanup of stale
local schedule files after the platform registration has already been handled.

`steward capabilities` is the agent-facing contract. Agents should inspect that
manifest and call the listed custody operations instead of constructing direct
`restic` or `resticprofile` shell commands. The manifest includes an
`agent_contract` and `operation_contracts` list with argv templates, required
inputs, side effects, success schemas, preflight requirements, and secret
handling rules. The `agent_contract.default_dogfood_policy` is selected by
default for registered dogfood agents such as `agent:codex`; branch, commit,
push, draft-PR, PR follow-up, and PR-check plans do not require a separate
policy file. Those plans still refuse protected branches and require the
`Agent-*` provenance trailers that distinguish agent authorship, delegated
policy, branch, verification, and coauthorship.

`steward command-plan` is the safer bridge from an agent request to an argv. It
accepts a constrained operation name plus typed inputs, then returns
`northroot.steward.command-plan.v0` with an argv array, missing-input checks,
warnings, side-effect metadata, and preflight readiness when `--execute` is
requested. Agent runtimes should prefer this over hand-binding templates. A
shell is not required for custody operations, and agents must not shell-join the
returned argv. The operation set includes steward custody operations, sanitized
legacy run imports, and the default dogfood branch/commit/push/draft-PR
workflow for registered agents.

When `--registry-state` and `--project-id` are supplied, `steward verify-state`
and `steward report` check both policy authorization and project destination
topology. A structurally valid registry still fails closed if a project has no
source destination, a source lacks objects or a destination, a replica lacks
required evidence, or the resume policy is not fail-closed for disconnected
storage and retention decisions.

When a policy names multiple destinations, steward renders the first destination
as the primary delegated `resticprofile` repository. Additional destinations are
reported in status, preflight, and capabilities as external destinations that
require explicit `verified_offsite_copy` evidence. They are not silently treated
as backed up just because the primary backup ran.

`steward offsite report --snapshot-id ...` is the agent-safe offsite/offload
gate. It lists additional destinations, private repository binding targets when
available, the external-delegated execution model, and whether
`verified_offsite_copy` evidence has been recorded for that snapshot. The report
returns non-zero while required offsite evidence is missing. Northroot still
does not implement a storage transport; use private deployment tooling such as
`rclone`, `restic copy`, or an external monitor to perform and verify the copy,
then record constrained evidence with `steward evidence record`.

`steward retention evaluate` is the profile-scoped retention gate. It returns a
`northroot.custody.retention-decision.v0` document and should be required before
any future prune, forget, or offload action.

`steward evidence report` derives conservative evidence from steward run
summaries. A successful delegated verification contributes `verified_snapshot`.
A restore drill contributes `restore_drill` only when the delegated restore
command succeeds and steward observes a restored target with at least one file,
byte count, and deterministic manifest hash. Offsite-copy evidence is not
inferred from backups or repository checks. It can be recorded with
`steward evidence record` after an external delegated tool or monitor verifies
the offsite copy for a specific snapshot. Recorded external evidence is only
consumed by retention checks for the matching `--snapshot-id`. Snapshot-filtered
reports also ignore unscoped delegated verify/restore evidence; run those
operations with `--snapshot-id` when they are meant to prove a retention gate.
External evidence import is intentionally limited to `verified_offsite_copy`;
repository-check and restore-drill evidence must come from their bounded steward
operations.

Public examples use symbolic references such as `workspace://...`,
`repository://...`, and `secret://...`.

Private deployments bind symbolic `secret://...` references with a
`northroot.custody.secret-bindings.v0` document. Steward uses those bindings
only to render resticprofile `password-command` entries. The command should be
unattended, explicit, and backed by a boring provider:

- `onepassword-cli`: use `op read ...` with `OP_SERVICE_ACCOUNT_TOKEN` supplied
  by either the scheduler environment or a `runtime_env` binding.
- `macos-keychain`: use `/usr/bin/security find-generic-password -w ...` with a
  pre-created generic password item and access granted for the background
  runner.
- `env-command`: use a local command maintained outside the public repo.

For background jobs, do not rely on interactive 1Password desktop approval or a
login shell exporting `OP_SERVICE_ACCOUNT_TOKEN`. Store a scoped 1Password
service-account token in the native keychain or another private automation
secret store, then map it through `runtime_env`:

```json
{
  "name": "OP_SERVICE_ACCOUNT_TOKEN",
  "provider": "macos-keychain",
  "command": [
    "security",
    "find-generic-password",
    "-w",
    "-s",
    "northroot-op-service-account-token",
    "-a",
    "northroot-steward"
  ],
  "interactive": false
}
```

At execution time steward materializes runtime env values only in the delegated
child process environment. It does not write them to scheduler templates,
resticprofile config, run summaries, or logs.

Private deployments bind symbolic `repository://...` references with a
`northroot.custody.repository-bindings.v0` document. Steward uses those bindings
only to render runnable resticprofile repository targets. Public policy keeps
repository targets symbolic; real local paths and remote repository URLs stay in
private deployment state.

Repository bindings may also include an optional noninteractive
`availability_check`:

```json
{
  "repository_ref": "repository://local-primary",
  "target": "repository-target://local-primary",
  "availability_check": {
    "mode": "probe-command",
    "command": ["storage-probe", "repository://local-primary"],
    "interactive": false,
    "timeout_seconds": 5
  }
}
```

When configured, `steward preflight` runs the probe as a read-only availability
check. A missing command, non-zero exit, or timeout marks the repository storage
unavailable and blocks unattended execution before delegated backup or restore
commands can run. This is the private deployment hook for disconnected external
storage, offline mounts, or unavailable network repositories.

Real machine paths, external-drive names, repository URLs, 1Password item
references, tokens, passwords, and live run summaries belong in private
deployment repos such as `Northroot-Labs`.
