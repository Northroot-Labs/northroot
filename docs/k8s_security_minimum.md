# Lean Kubernetes Security and Secrets Plan

This keeps secrets out of images/manifests, separates environments, and stays open-source friendly. Pair with `docs/secrets.md` for devcontainer guidance.

## Environment Layout
- Namespaces per env: `dev`, `stage`, `prod`; per-env service accounts; deny cross-namespace secret reads via RBAC.
- Per-env secret backend scope and keys: e.g., AWS `ssm://project-dev/*` + `kms-dev`, `ssm://project-prod/*` + `kms-prod`.
- Per-env ingress/DNS; no prod credentials in lower environments.

## Secrets Options (pick one to start)
- External Secrets Operator (ESO) + cloud secret manager (recommended on cloud):
  - One `SecretStore` per env, pointing to that env’s project/region/key.
  - Per-app `ExternalSecret` in its namespace; ESO writes a native `Secret`.
  - Rotation: let the backend rotate; ESO resyncs (set a reasonable refreshInterval).
- SOPS + age (portable/GitOps-friendly):
  - Per-env age key; store only public key in repo.
  - Encrypt manifests (or values) with SOPS; decrypt in CI with short-lived creds; apply from tmpfs.
  - Rotation = new age key, re-encrypt, update CI secret.

## Minimal Examples

### External Secrets Operator (dev namespace)
```yaml
apiVersion: external-secrets.io/v1beta1
kind: SecretStore
metadata:
  name: dev-secrets
  namespace: dev
spec:
  provider:
    aws:
      service: SecretsManager
      region: us-east-1
      auth:
        jwt:
          serviceAccountRef:
            name: dev-app-sa
---
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: app-secrets
  namespace: dev
spec:
  refreshInterval: 1h
  secretStoreRef:
    name: dev-secrets
  target:
    name: app-secrets
    creationPolicy: Owner
  data:
    - secretKey: DB_URL
      remoteRef:
        key: project-dev/app/db-url
```

### SOPS + age workflow (per env)
- Generate key: `age-keygen -o agekey-dev.txt`; keep private key out of repo, commit the public stanza.
- Encrypt: `sops --encrypt --age <age-recipient> values-dev.yaml > values-dev.sops.yaml`.
- CI: export age key from secret manager into a tmpfs file, `sops --decrypt values-dev.sops.yaml | kubectl apply -f -`, then wipe tmpfs.

## Guardrails (small set)
- Pod security: use `PodSecurity` admission at `restricted` level for all namespaces.
- Kyverno starter policies:
  - Deny privileged/hostNetwork/hostPID/hostIPC.
  - Deny hostPath volumes.
  - Require image digests (no mutable tags).
  - Optionally require trusted registries.
- NetworkPolicies:
  - Default deny ingress/egress per namespace; allow only needed destinations (e.g., secret backend, database, ingress controller).

## Image and Supply Chain
- Build with BuildKit; inject secrets with `--secret` to avoid layer leaks.
- Sign images with cosign (optional starter), or at least pin digests in manifests.
- Avoid Docker socket mounts; use dedicated builders (kaniko/buildkit) with scoped creds.

## Access Control
- Human access: short-lived kubeconfigs via cloud IAM/OIDC if available; otherwise time-bound service account tokens.
- Service accounts: per-app, namespace-scoped, least privilege; only those identities can read their secrets.

## Ingress/TLS
- cert-manager with DNS-01 per env; distinct certificates per env domains.

## Rollout Sequence (minimal)
1) Create namespaces `dev`, `stage`, `prod` with `restricted` PodSecurity + default-deny NetworkPolicies.
2) Install ESO (or adopt SOPS path) and create per-env `SecretStore`.
3) Add per-app `ExternalSecret` (or SOPS-encrypted values) in each namespace.
4) Apply Kyverno starter policies (privilege/hostPath/digest).
5) Ensure image references are pinned to digests; deploy.

