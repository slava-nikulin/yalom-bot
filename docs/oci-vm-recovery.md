# OCI k3s VM recovery — manual runbook and OpenTofu reference

## Goal

Make the single-node Yalom k3s server reproducible after an OCI Always Free Compute instance is stopped/reclaimed/terminated. This document is **both** a manual recovery runbook and an implementation reference for OpenTofu. Scope is **the VM and its connections to already-existing OCI resources**; the separate MySQL DB System is explicitly out of scope.

Do not create a new VCN, NLB, reserved public IP, Vault, MySQL DB System, or Object Storage bucket during recovery. The aim is to restore the application behind the existing NLB while preserving `141.147.3.145`.

> **Status:** Configuration recorded from live OCI CLI inspection (2026-10-14) and current repo. **Not yet disaster-recovery-tested**. Commands marked as mutations are a runbook for a future incident, **not** instructions to run against the healthy instance now. Substitute actual OCIDs using a secure local environment; do not publish credentials, SSH private keys, vault contents, or state.

## Known configuration (verified)

| Component | Existing configuration | Recovery treatment |
| --- | --- | --- |
| Region | `eu-frankfurt-1` | Preserve |
| Compartment | Tenancy root | Reuse |
| Availability domain | `LrFf:EU-FRANKFURT-1-AD-3` | Preferred; try a different suitable placement if A1 capacity unavailable |
| VM | `VM.Standard.A1.Flex`, 2 OCPU, 12 GB RAM | Recreate, or restart existing if only stopped |
| OS | Launched from OCI image (image OCID known locally; OS version **not yet independently verified**) | Check image/arch before recovery |
| Boot volume | 47 GB, attached; no boot-volume backups found | New volume on clean build; reused volume only as an explicit alternative if preserved |
| Other volumes | No additional OCI Block Volumes | None to reattach |
| VM primary VNIC | Public subnet, currently private `10.0.1.88`, ephemeral public IP | New VNIC; allocate a new private IP/ephemeral public IP for SSH |
| Subnet | Same subnet used by NLB (`10.0.1.x`) | Reuse existing subnet OCID |
| VM NSG | `yalom-k3s-nodes` | Add new primary VNIC to existing NSG at launch |
| NLB | `yalom-edge`, reserved public IPv4 `141.147.3.145` | **Keep** |
| NLB NSG | `yalom-nlb` | **Keep** |
| NLB listeners | TCP `80` → `k3s-http`; TCP `443` → `k3s-https` | Keep |
| NLB backends | Currently target **Private IP OCID**, corresponding to `10.0.1.88`, on both ports | Replace both backend targets with new Private IP OCID |
| NLB health checks | TCP ports 80/443, 10 s interval, 3 s timeout, 3 retries | Keep |
| VM defined tags | `Yalom.Role=app-node`; `Yalom.DeploymentTarget=production-k3s-server` | Restore both |
| OCI Vault | External Secrets Operator uses OCI Instance Principal | Keep; validate Dynamic Group/IAM policy for new VM |
| Release | `release.yml` discovers exactly **one RUNNING** VM by `Yalom.DeploymentTarget`, uploads `production/deploy.sh` into `yalom-bot-release` bucket, runs via OCI Instance Agent | Preserve; validate Instance Agent + Run Command privileges |
| k3s | Single-node k3s; `cluster/k3s/bootstrap/bootstrap.sh` configures k3s, Traefik, ESO, cert-manager, Gateway API | Re-run bootstrap |
| DB | Separate active `yalom-mysql` OCI `MySQL.Free` DB System; endpoint `10.0.2.78:3306` | **Do not modify**. Confirm app can reconnect |

### Existing repository entry points

- `cluster/k3s/bootstrap/bootstrap.sh`: remote SSH bootstrap entry point.
- `cluster/k3s/bootstrap/bootstrap-server.sh`: pinned k3s version, Traefik, cert-manager, ESO, OCI Run Command sudo configuration, readiness checks.
- `cluster/k3s/cert-manager/certificate.yaml`: Let's Encrypt certificate for `141.147.3.145`.
- `deploy/k8s/base/config.env`: fixed MiniApp and Telegram webhook URLs (same reserved NLB public IP).
- `deploy/k8s/overlays/oci-k3s/`: application manifests + ExternalSecret with `InstancePrincipal` authentication.
- `.github/workflows/release.yml`: builds GHCR images, uploads a release artifact to OCI Object Storage, locates instance by tag, deploys via OCI Run Command.
- `.github/workflows/telegram-setup.yml`: explicitly reconfigure Telegram only if webhook/MiniApp configuration needs it.

## Manual recovery runbook — clean new VM path

### A. Preflight — **read-only**

1. Check whether the previous instance is just `STOPPED`, rather than terminated. If still present and recoverable, starting it may be simpler than replacing it.
2. Check whether its boot volume remains usable. Existing backups: **none** at audit time. Do not automatically delete retained boot volumes.
3. Save the existing NLB backend names and target IDs before any changes; use `oci nlb backend list` for both `k3s-http` and `k3s-https`.
4. Verify the source image is still available and appropriate for AArch64 Ubuntu: `oci compute image get --image-id "$IMAGE_ID"`. If not, select a compatible replacement image and document it.
5. Verify IAM Dynamic Group matching rule and Vault permissions (previous tag alone is **not** evidence that rule is correct). Check SSH ingress access, subnet routing, and VCN-to-MySQL connectivity.
6. Verify available A1 capacity and free quota. Failure to provision due to capacity is a possible external blocker.
7. **Do not run a second release while two RUNNING instances share** `Yalom.DeploymentTarget=production-k3s-server`: `release.yml` requires exactly one match.

Assume the operator has OCI CLI configured and variables with actual OCIDs from the current tenancy, kept outside the public repository:

```bash
export OCI_CLI_REGION=eu-frankfurt-1
export COMPARTMENT_ID='<tenancy-or-compartment-ocid>'
export AD='LrFf:EU-FRANKFURT-1-AD-3'
export SUBNET_ID='<existing-subnet-ocid>'
export K3S_NSG_ID='<yalom-k3s-nodes-nsg-ocid>'
export NLB_ID='<yalom-edge-nlb-ocid>'
export IMAGE_ID='<compatible-ubuntu-aarch64-image-ocid>'

oci compute image get --image-id "$IMAGE_ID" \
  | jq '.data | {name: ."display-name", os: ."operating-system", version: ."operating-system-version", state: ."lifecycle-state"}'

oci iam dynamic-group list --compartment-id "$COMPARTMENT_ID" --all \
  | jq '.data[] | {name, matching_rule: ."matching-rule"}'

oci iam policy list --compartment-id "$COMPARTMENT_ID" --all \
  | jq '.data[] | {name, statements}'
```

**Open questions before rehearsal:** exact OS image/version; Dynamic Group matching rule and applicable IAM policy; SSH ingress/security list details; OCI Instance Agent Run Command availability; whether all required k3s state is reproducible from Git.

### B. Provision the replacement instance — **MUTATION; incident only**

Launch an A1 shape in the **existing** subnet and NSG, with both defined tags, a public SSH key, and a public IP for the existing SSH bootstrap path:

```bash
NEW_INSTANCE_ID="$(oci compute instance launch \
  --compartment-id "$COMPARTMENT_ID" \
  --availability-domain "$AD" \
  --display-name yalom-k3s \
  --shape VM.Standard.A1.Flex \
  --shape-config '{"ocpus":2,"memoryInGBs":12}' \
  --image-id "$IMAGE_ID" \
  --subnet-id "$SUBNET_ID" \
  --nsg-ids "[\"$K3S_NSG_ID\"]" \
  --assign-public-ip true \
  --ssh-authorized-keys-file "$HOME/.ssh/id_ed25519.pub" \
  --defined-tags '{"Yalom":{"Role":"app-node","DeploymentTarget":"production-k3s-server"}}' \
  --wait-for-state RUNNING \
  --query data.id --raw-output)"

printf 'New VM: %s\n' "$NEW_INSTANCE_ID"
oci compute instance list-vnics --instance-id "$NEW_INSTANCE_ID" \
  | jq '.data[] | {id, private_ip: ."private-ip", public_ip: ."public-ip", nsg_ids: ."nsg-ids"}'
```

**Boot-volume size caveat:** current volume reports **47 GB**. The OCI Always Free guide describes a 47 GB default/minimum, while recent generic CLI `--boot-volume-size-in-gbs` reference says 50 GB minimum. The example deliberately **does not request an explicit boot volume size**: inspect the newly provisioned volume and applicable Always Free usage. Do not automatically resize the current volume or exceed free storage allowance.

If a retained boot volume is intentionally reused instead of a clean installation, use the documented `--source-boot-volume-id` alternative **instead of** `--image-id`; check volume state, AD and potential stale k3s/network configuration. This scenario requires a separate rehearsal.

### C. Identify the new Private IP **resource** — **read-only**

The current NLB is attached to a `ocid1.privateip...`, not a literal IP string. Resolve the new one:

```bash
NEW_VNIC_ID="$(oci compute instance list-vnics --instance-id "$NEW_INSTANCE_ID" \
  | jq -r '.data[0].id')"

oci network private-ip list --vnic-id "$NEW_VNIC_ID" --all \
  | jq '.data[] | {id, ip_address: ."ip-address", primary: ."is-primary"}'

NEW_PRIVATE_IP_ID="$(oci network private-ip list --vnic-id "$NEW_VNIC_ID" --all \
  | jq -r '.data[] | select(."is-primary" == true) | .id')"

test -n "$NEW_PRIVATE_IP_ID" || { echo 'Primary private IP was not found' >&2; exit 1; }
```

### D. Attach NLB backends — **MUTATION; before bootstrap completes**

The `cluster/k3s/bootstrap/bootstrap-server.sh` waits for an ACME HTTP-01 certificate issued through the NLB's public IP. Therefore a new backend for **port 80 must already be registered** by the time the bootstrap reaches certificate issuance; otherwise the public challenge cannot reach Traefik. Register **both** backends before starting bootstrap. An initially unhealthy backend is expected until Traefik starts.

```bash
for spec in 'k3s-http 80' 'k3s-https 443'; do
  read -r SET_NAME PORT <<< "$spec"
  oci nlb backend create \
    --network-load-balancer-id "$NLB_ID" \
    --backend-set-name "$SET_NAME" \
    --target-id "$NEW_PRIVATE_IP_ID" \
    --port "$PORT" \
    --wait-for-state SUCCEEDED
done
```

This runbook is primarily for **loss of the old VM**. Live blue/green replacement is **not** handled here: adding new backends before app rollout could route production traffic to an incompletely deployed node.

### E. Bootstrap k3s — **MUTATION; incident only**

Find the new ephemeral VM public IP and ensure SSH access is allowed from the operator's source IP. Use the same SSH user/key assumptions as the existing bootstrap script:

```bash
NEW_PUBLIC_IP="$(oci compute instance list-vnics --instance-id "$NEW_INSTANCE_ID" \
  | jq -r '.data[0]."public-ip"')"

test -n "$NEW_PUBLIC_IP" && test "$NEW_PUBLIC_IP" != null

./cluster/k3s/bootstrap/bootstrap.sh "ubuntu@$NEW_PUBLIC_IP"
```

Expected: k3s Ready, Traefik DaemonSet Ready, ESO/cert-manager installed, Gateway programmed, Let's Encrypt IP certificate Ready, OCI Run Command sudoers installed. The script configures these resources; it does **not** deploy the latest application itself.

Check new NLB health after Traefik is running:

```bash
for spec in 'k3s-http 80' 'k3s-https 443'; do
  read -r SET_NAME PORT <<< "$spec"
  oci nlb backend-health get \
    --network-load-balancer-id "$NLB_ID" \
    --backend-set-name "$SET_NAME" \
    --backend-name "${NEW_PRIVATE_IP_ID}:${PORT}" \
    | jq '.data'
done
```

### F. Release and validate — **MUTATION for release; incident only**

1. Ensure exactly **one** RUNNING VM has `Yalom.DeploymentTarget=production-k3s-server`. Remove that tag from a still-running old VM if necessary; do not delete a recoverable instance merely to resolve the selector.
2. Confirm OCI Instance Agent **Run Command** is available on the new VM and that the bootstrap's sudoers file permits `ocarun` to run k3s.
3. Trigger the existing **Release** GitHub Actions workflow (`main`, `production`). It should find the new VM, upload `deploy.sh` to OCI Object Storage and execute it through OCI Run Command.
4. Check migration success, ExternalSecret synchronization with Vault, application rollout, TLS certificate readiness, and access from Pods to the **existing** MySQL endpoint.
5. Test external `https://141.147.3.145/miniapp/`, Telegram webhook, interactive MiniApp and sending a manual nudge. The backend `/health` is internal (not directly exposed by the current `HTTPRoute`). TCP health checks alone are insufficient.
6. Only after the new instance is validated, remove old NLB backends. Obtain `backend-name` values from `oci nlb backend list` before deletion; the names are `${OLD_PRIVATE_IP_OCID}:80` and `${OLD_PRIVATE_IP_OCID}:443` in the currently observed configuration.

**Old backend removal (manual, confirm names first):**

```bash
for SET_NAME in k3s-http k3s-https; do
  oci nlb backend list \
    --network-load-balancer-id "$NLB_ID" \
    --backend-set-name "$SET_NAME" \
    | jq '.data.items[] | {name, target_id: ."target-id", ip_address: ."ip-address", port}'
done

# After positively identifying the OLD names, per backend:
# oci nlb backend delete --network-load-balancer-id "$NLB_ID" \
#   --backend-set-name '<k3s-http|k3s-https>' --backend-name '<old-name>'
```

Do not delete NLB **backend sets**, listeners, reserved IP, or any database-related resource.

## OpenTofu implementation specification

### Scope, boundaries, and ownership

- Manage **replaceable compute** using `oci_core_instance`: AD/shape/image, OCPU/memory, boot volume, existing subnet/NSG, SSH/cloud-init metadata, both defined tags, and agent settings.
- Manage two **NLB backend memberships** with `oci_network_load_balancer_backend` (sets `k3s-http`/`k3s-https`, ports 80/443), resolving the current instance's Private IP resource OCID programmatically. The current API accepts `target_id` with a Private IP OCID. Derive it via the VNIC/Private IP data sources or a provider-supported equivalent rather than hardcoding `10.0.1.88`.
- Treat existing VCN, subnet, `yalom-k3s-nodes` NSG, `yalom-nlb` NSG, NLB/listeners/backend sets, reserved NLB public IP, OCI Vault, IAM policies, Object Storage bucket, and MySQL DB System as **pre-existing/external dependencies**, using data sources or variables. If importing those later, do it as a separate safely reviewed task.
- Keep secret *values*, SSH private keys and credentials **out of Git and OpenTofu state**. Reference OCI/Vault identifiers as needed. SSH **public** key may be configured as input.
- Keep bootstrap and application deployment separate initially: OpenTofu provisions OCI resources; `cluster/k3s/bootstrap/bootstrap.sh` provisions guest/k3s; GitHub Actions deploys the app. Automating bootstrap with cloud-init or a reusable OCI Run Command step can be future work.
- Pin OCI provider and OpenTofu versions; define `outputs` for instance OCID, VM public IP, private IP/private-IP OCID, and NLB backend names.
- Establish safe state storage (prefer restricted remote state with backup, appropriate access controls and locking where supported), plus reviewable `tofu plan`; **never blindly run `apply` against existing resources**. Do not import/recreate the existing NLB and MySQL by accident.
- Replacements must not destroy persistent resources; ensure dependency graph handles creating/updating backend memberships and NLB readiness appropriately.

### Desired structure (illustrative, not implementation)

```text
infra/opentofu/
  versions.tf       # OpenTofu/OCI provider constraints
  provider.tf       # OCI auth/region; no secrets committed
  variables.tf      # compartment, AD, subnet/NSG/NLB refs, image, SSH pubkey
  compute.tf        # oci_core_instance: 2 OCPU / 12 GB, two tags
  nlb-backends.tf   # two oci_network_load_balancer_backend resources
  outputs.tf        # instance / VNIC / IP / backend outputs
  README.md         # plan/apply/recovery and state handling
```

### Acceptance criteria

- [ ] Recovery procedure documented and successfully **rehearsed** in a safe environment or during an intentional recovery.
- [ ] New VM is provisioned using existing subnet and `yalom-k3s-nodes`, with both defined tags and appropriate SSH/Instance Agent settings.
- [ ] Two NLB backends dynamically target the **new primary Private IP OCID**; no old dead backends remain.
- [ ] Reuses existing `yalom-edge` and its reserved public IP `141.147.3.145`; does not recreate VCN, MySQL, Vault, NLB, Object Storage or IAM policies.
- [ ] k3s bootstrap succeeds, including public IP ACME validation with NLB routing in place.
- [ ] GitHub Actions `release.yml` finds exactly one RUNNING production VM and successfully deploys.
- [ ] Telegram webhook/MiniApp/manual message end-to-end smoke test passes, database unchanged.
- [ ] `tofu plan` is reviewed, versions/state are documented, secrets are not committed, and re-running the workflow does not cause destructive infrastructure drift.

### Risks / deferred questions

- A1 capacity in Frankfurt is not guaranteed at replacement time. OpenTofu cannot eliminate capacity shortages.
- No boot-volume backup exists in the audit. Clean rebuilding depends on the Git-tracked bootstrap and externally persisted app state.
- OCI's Always Free guide mentions a 47 GB boot-volume default/minimum, but the current generic CLI flag docs mention 50 GB: verify actual default/cost before explicitly sizing the volume.
- Verify actual IAM Dynamic Group matching rule, policy statements, network security lists/SSH access and Run Command agent state; these were not shown in the inspection output.
- Replacement is **not zero-downtime**; no second VM or HA promise is implied.
- Do not include MySQL provisioning/recovery, distributed Kubernetes, automated NLB creation, backup policy creation, scheduled rotation or full tenancy IaC in this recovery scope.

## References

- [OCI CLI: Compute instance launch](https://docs.oracle.com/en-us/iaas/tools/oci-cli/latest/oci_cli_docs/cmdref/compute/instance/launch.html)
- [OCI CLI: NLB backend create](https://docs.oracle.com/en-us/iaas/tools/oci-cli/latest/oci_cli_docs/cmdref/nlb/backend/create.html)
- [OCI CLI: NLB backend health](https://docs.oracle.com/en-us/iaas/tools/oci-cli/latest/oci_cli_docs/cmdref/nlb/backend-health/get.html)
- [OCI provider: NLB backend](https://registry.terraform.io/providers/oracle/oci/latest/docs/resources/network_load_balancer_backend)
- [OCI Always Free resources](https://docs.oracle.com/en-us/iaas/Content/FreeTier/freetier_topic-Always_Free_Resources.htm)