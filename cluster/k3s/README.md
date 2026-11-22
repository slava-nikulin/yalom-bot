# Yalom k3s cluster

This directory contains the desired configuration and bootstrap
procedure for the self-managed Yalom Kubernetes cluster.

## Responsibilities

The cluster layer manages:

- k3s
- Traefik
- Gateway API
- External Secrets Operator
- cert-manager
- shared Gateway resources

Application resources are stored separately in:

deploy/k8s/

## Bootstrap

A fresh Ubuntu VM can be converted into a Yalom k3s server with:

    ./cluster/k3s/bootstrap/bootstrap.sh ubuntu@<public-ip>

SSH is used only as a transport.

The bootstrap process:

    local Git configuration
        -> SSH
        -> Ubuntu VM
        -> install/configure k3s
        -> configure OCI Run Command access

Bootstrap is idempotent and manages only the host and k3s installation.
The GitHub Release workflow reconciles HelmCharts, Traefik, Gateway,
cert-manager, External Secrets and application manifests.

## k3s

The pinned k3s version is defined in:

    bootstrap/bootstrap-server.sh

k3s persistent configuration:

    config/server.yaml

On the server this becomes:

    /etc/rancher/k3s/config.yaml

## Traffic path

    OCI NLB TCP/80  -> node :8080
    OCI NLB TCP/443 -> node :8443
        -> Traefik DaemonSet hostPort
        -> Traefik HTTPS containerPort :443
        -> Gateway API
        -> application HTTPRoute
        -> Service
        -> Pod

k3s ServiceLB is disabled.

Traefik runs as a DaemonSet: one Traefik Pod per cluster node.

