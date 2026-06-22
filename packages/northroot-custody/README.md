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
nr-custody steward run --state /tmp/northroot-steward-example --snapshot-id snap-001
nr-custody steward run \
  --state /tmp/northroot-steward-example \
  --registry-state /tmp/northroot-steward-registry \
  --project-id project/example \
  --snapshot-id snap-001 \
  --execute
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
nr-custody steward registry recover --state /tmp/northroot-steward-registry --public-safe
nr-custody steward schedule create \
  --state /tmp/northroot-steward-example \
  --scheduler launchd \
  --operation run \
  --every-minutes 60
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
hand-editing generated config.

`steward verify-state` is the read-only aggregate gate for agents and daemons.
It composes status, preflight, the capability manifest, schedule metadata, and
snapshot-scoped evidence into a `northroot.steward.state-verification.v0`
document. With `--snapshot-id`, it also evaluates whether recorded evidence is
sufficient for that snapshot's retention gate. It does not execute backups,
install schedules, run restore drills, or record evidence.

`steward report` is the read-only operator and agent report. It composes status,
preflight, schedule state, evidence, offsite copy status, retention readiness,
and recommended next actions into `northroot.steward.report.v0`. It is not a
gate and does not execute backups, install schedules, run restore drills, or
record evidence.

`steward registry` is the durable state-management surface for the service
registry. `registry init` validates and installs a public-safe registry
document. Mutation commands append object custody entries, project/object
permissions, registered projects, destinations, source bindings, replicas, and
legacy import records through atomic writes. If a machine dies or the process is
interrupted while a registry mutation lock exists, later mutations fail closed
until `registry recover` validates the current registry and records the
interrupted operation. Recovery removes the lock only when the registry still
validates.

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
files only when the schedule is not marked installed. Use `schedule delete
--force` only for operator-confirmed cleanup of stale generated files after the
platform registration has already been removed. Refused deletion returns a
non-zero exit status so scripts and agents cannot mistake a blocked cleanup for
success. `--skip-preflight` is available for explicit operator-controlled
exceptions. Executed operations fail closed when preflight is not ready and
still write an auditable run summary with `delegated-preflight-failed` status.
Scheduled operations do not invent a snapshot id; treat their evidence as
general custody health unless the generated command is deliberately bound to a
specific snapshot.

The generated schedule records its `runner_command` and preflight checks that
the command is executable in the current environment. For unattended launchd or
systemd jobs, prefer an absolute runner path or a deliberately installed `nr`
shim instead of relying on an interactive shell PATH. Schedule rendering parses
the runner command with shell rules, appends steward arguments, shell-quotes
paths, and escapes launchd XML so state directories with spaces or XML-sensitive
characters do not break unattended execution. If the runner executable path
contains spaces, quote it inside `--runner-command`, for example
`--runner-command "'/opt/Northroot Tools/nr' steward"`. Generated schedule
templates are also hashed in `schedule.json`; preflight fails if a rendered
launchd plist or systemd unit/timer drifts after creation.

`steward capabilities` is the agent-facing contract. Agents should inspect that
manifest and call the listed custody operations instead of constructing direct
`restic` or `resticprofile` shell commands. The manifest includes an
`agent_contract` and `operation_contracts` list with argv templates, required
inputs, side effects, success schemas, preflight requirements, and secret
handling rules.

`steward command-plan` is the safer bridge from an agent request to an argv. It
accepts a constrained operation name plus typed inputs, then returns
`northroot.steward.command-plan.v0` with an argv array, missing-input checks,
warnings, side-effect metadata, and preflight readiness when `--execute` is
requested. Agent runtimes should prefer this over hand-binding templates. A
shell is not required for custody operations, and agents must not shell-join the
returned argv.

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

Real machine paths, external-drive names, repository URLs, 1Password item
references, tokens, passwords, and live run summaries belong in private
deployment repos such as `Northroot-Labs`.
