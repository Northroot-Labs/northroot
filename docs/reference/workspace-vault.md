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

## Object Store Facade

The vault uses an ObjectStore facade so storage backends can change without
changing workspace semantics. V0 implements the local filesystem backend. Cloud
backends such as GCS or S3 can preserve the same prefixes later.

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
  vault/
    raw/
    derived/
    indexes/
    logs/
    receipts/
    manifests/
      workspace.json
      vault-boundary.json
      connections/
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

## Gmail Connection v0

`northroot connect gmail --workspace <path> --mode readonly` records a
non-secret connection manifest under `vault/manifests/connections/gmail.json`.

The manifest records provider, mode, scopes, workspace binding, and an auth
profile reference. It does not store OAuth tokens, app passwords, credentials,
or secret values.

Provider authentication and browser login remain delegated to provider-specific
tools such as the Google Workspace CLI. Product-specific use, including APD data
landing and reconciliation, belongs in ClearlyOps or reusable skills that bind
to this workspace vault.
