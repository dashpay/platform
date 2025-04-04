# Troubleshooting Guide

This guide addresses common issues that may occur when using Dashmate.

## Doctor

The `doctor` command is a diagnostic tool that can help identify issues with your Dashmate setup.
It checks the configuration and state of your node, providing a list of potential problems and solutions.

```bash
dashmate doctor
```

## Common Issues

### Node Not Running

**Problem:** You see `[FAILED] Node is not running` error message.

**Solution:** Force stop the node before trying to start it again:

- For a single node (fullnode/masternode):
  ```bash
  $ dashmate stop --force
  ```

- For a group of nodes (local):
  ```bash
  $ dashmate group stop --force
  ```

### Running Services Preventing Start

**Problem:** `Running services detected. Please ensure all services are stopped for this config before starting`

**Solution:** Some nodes are still running and preventing dashmate from starting properly. This often occurs after a command exits with an error. Force stop the nodes:

- For a single node:
  ```bash
  $ dashmate stop --force
  ```

- For a group of nodes:
  ```bash
  $ dashmate group stop --force
  ```

### External IP Configuration Error

**Problem:** `externalIp option is not set in base config`

**Solution:** This can happen when switching between major versions, making the config incompatible. Perform a manual reset and run setup again:

```bash
docker stop $(docker ps -q)
docker system prune
docker volume prune
rm -rf ~/.dashmate/
dashmate setup
```

### TypeError in Plugin

**Problem:** `TypeError Plugin: dashmate: Cannot read properties of undefined (reading 'dash')`

**Solution:** This can occur if other `.yarnrc` and `node_modules` directories exist in parent directories. Check your home directory for any `.yarnrc` and `node_modules`, delete them all and try again.

### Manual Reset

If the local setup is corrupted and a hard reset does not fix it, you can perform a manual reset:

```bash
docker stop $(docker ps -q)
docker system prune
docker volume prune
rm -rf ~/.dashmate/
```

After the manual reset, you'll need to set up your node again from scratch.

## Diagnostic Tools

### Doctor Command

Dashmate includes a diagnostic tool that can help identify issues:

```bash
$ dashmate doctor
```

This will analyze your node configuration and state, providing a list of potential problems and solutions.

### Diagnostic Report

To gather detailed diagnostic information for troubleshooting, use:

```bash
$ dashmate doctor report
```

This creates an archive with system information, node configuration, and service logs with sensitive data obfuscated.

## Getting Help

If you're still experiencing issues:

1. Check the [Dash Platform documentation](https://docs.dash.org/projects/platform)
2. Visit the [Dash Discord](https://chat.dash.org/) for community support
3. Open an issue on [GitHub](https://github.com/dashpay/platform/issues/new/choose)
