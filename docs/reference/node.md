# Node Substrate

A Northroot node is a durable identity for a custody and execution boundary.
It is not a repository, and it is not automatically a whole physical machine.

The default local node is user-scoped:

```text
~/.northroot/node/
  node.json
  journal/
  state/
  vault/objects/
  tmp/
```

Initialize it with:

```bash
northroot node init --slug local-node
```

Inspect it with:

```bash
northroot node status --json
```

## Identity

The manifest separates durable identity from human aliases:

- `node_id`: generated stable identifier, for example `node:local-node-...`
- `slug`: human alias, scoped and mutable
- `scope`: custody boundary, one of `user-machine`, `org`, `service`, or
  `workspace`

Use slugs for operator ergonomics. Use generated ids for durable references.

## Storage

Node manifests describe storage through URI bindings:

```json
{
  "index": {
    "kind": "sqlite",
    "uri": "sqlite://state/node.db"
  },
  "object_stores": [
    {
      "id": "local",
      "kind": "fs",
      "uri": "fs://vault/objects",
      "role": "primary"
    }
  ]
}
```

The index stores metadata: node bindings, workspace registrations, run state,
receipt indexes, object references, and policy checks.

Object stores hold payloads: blobs, snapshots, manifests, and other artifacts
addressed by content or object id.

Local nodes start with SQLite and filesystem storage. Cloud and deployment
layers may bind Postgres, S3, R2, restic, Docker, Kubernetes, or OpenTofu around
the same manifest contract. Those deployment choices are not kernel semantics.

## Workspace Boundary

A workspace may have its own `.northroot/workspace.json`, but repo-local
workspace metadata must not become the node root. Concrete machine paths,
mounted volumes, private project registrations, and operational receipts belong
in the node index or private deployment profile, not in public git history.
