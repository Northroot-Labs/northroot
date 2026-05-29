# Workspace Vault v0

Status: Incubating CLI contract

## Boundary

A Northroot workspace is the local execution and configuration boundary for
skills, connections, receipts, local agent state, and policy references.

A workspace vault is the data boundary for a user, department, client, or
project. It is where local and edge workflows land raw data, derived artifacts,
indexes, logs, receipts, and manifests.

A workspace vault is not a password vault or secret manager. Credential values,
tokens, private keys, browser cookies, and provider secrets stay in external
stores such as 1Password, provider CLIs, Secret Manager, or OS keychains. The
workspace vault may record non-secret references to those stores.

Northroot owns portable workspace machinery. Product or service profiles belong
beside it, not inside the substrate. For example, `.northroot/` may describe
generic vault, connection, skill, agent, receipt, and company registry state,
while `.clearlyops/` may describe APD, CropTrak, reconciliation, review queues,
service policy, and ClearlyOps-specific customer workflow defaults.

If another company could use the same shape with its own domain, mailbox,
policies, and customers, the state belongs in `.northroot/`. If it assumes
ClearlyOps as the service operator, APD as a named customer/company workflow,
or ClearlyOps product posture, it belongs in `.clearlyops/`.

## Object Store Facade

The vault uses an ObjectStore facade so storage backends can change without
changing workspace semantics. V0 implements the local filesystem backend. Cloud
backends such as GCS or S3 can preserve the same prefixes later.

Objects are storage-level byte records with backend-relative paths and
content-addressed metadata. Artifacts are semantic work products, such as a
generated export, receipt bundle, extracted document, or proof package. An
artifact may be backed by one or more objects or content refs, but an object is
not an artifact by itself.

Required object store operations:

- `put`: store immutable bytes and return content-addressed metadata.
- `get`: read bytes by backend-relative path.
- `exists`: check for a backend-relative path.
- `list_prefix`: list backend-relative paths under a prefix.
- `write_manifest` and `read_manifest`: store named JSON manifests.

Raw immutable bytes should be content-addressed. Named paths are reserved for
manifests, indexes, logs, receipts, and derived views.

## Local Layout

`northroot workspace init --name <name> --root <path>` creates:

```text
<path>/
  .northroot/
    workspace.json
    vault.json
    connections/
    skills/
    agents/
    registry/
      companies.json
    vault/
      raw/
      derived/
      indexes/
      logs/
      receipts/
      manifests/
        workspace.json
        vault-boundary.json
```

Prefix roles:

- `raw/`: immutable landed source data.
- `derived/`: normalized records, extracted text, embeddings, and generated
  artifacts.
- `indexes/`: SQLite, vector, and other local index state.
- `logs/`: local operational logs.
- `receipts/`: import, run, and verification receipts.
- `manifests/`: source refs, hashes, sensitivity, ownership, and boundary
  metadata.

`connections/`, `skills/`, `agents/`, and `registry/` are generic local
workspace state. They are not vault object-store prefixes. Connection files
record provider references such as `provider:gws:profile:default`; they must not
store OAuth tokens, passwords, cookies, private keys, or credential values.

`registry/companies.json` is a local-first company/customer registry. It may
record durable local company IDs, display names, aliases, source refs, and
external refs. Short aliases such as APD are aliases or product bindings, not
Northroot primitives. Profile-specific use belongs in a profile pack such as
`.clearlyops/apd/company-binding.json`.

Product profiles may sit beside `.northroot/`:

```text
<path>/
  .clearlyops/
    profile.json
    apd/
      company-binding.json
      workspace-template.json
    reconciliation/
      rules.toml
    intake/
      mailbox-routing.toml
    review-queues/
      apd.json
    service-policy.toml
```

## Gmail Connection v0

`northroot connect gmail --workspace <path> --mode readonly` records a
non-secret connection manifest under `.northroot/connections/gmail.json`.

The manifest records provider, mode, scopes, workspace binding, and an auth
profile reference. It does not store OAuth tokens, app passwords, credentials,
or secret values.

Provider authentication and browser login remain delegated to provider-specific
tools such as the Google Workspace CLI. Product-specific use, including APD data
landing and reconciliation, belongs in ClearlyOps or reusable skills that bind
to this workspace vault.

## Local, Edge, and Cloud Threshold

Local is the default. Laptop-runnable ingestion, reconciliation, local indexes,
receipts, and operator review should work against `.northroot/` and the local
vault first.

Use edge execution when work needs an always-on site or department node, shared
local access, heavier local models, or scheduled near-data processing.

Use cloud services when the requirement is durable replication, scheduled
backups, provider webhooks, remote collaboration, high availability,
cross-device sync, or compute that no longer fits the local/edge boundary.
Cloud backends can later implement the same ObjectStore facade; v0 only
implements the local filesystem backend.
