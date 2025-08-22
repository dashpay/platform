# status host

The `status host` command displays system information about the host running Dashmate.

## Usage

```bash
dashmate status host [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-c, --config=<name>` | Configuration to use | *Uses default config if not specified* |
| `--format=<format>` | Display output format (`plain`, `json`, `yaml`) | `plain` |

## Description

This command provides detailed information about the host system where Dashmate is running.

It displays system metrics including:
- Hostname
- System uptime
- Platform (operating system)
- Architecture
- Username
- Memory information
- CPU information
- IP address

This information is useful for diagnosing system-related issues, verifying system requirements, and for providing system details when seeking support.

## Examples

```bash
# Show host information
dashmate status host

# Show host information in JSON format
dashmate status host --format=json
```

Example output:
```
Hostname: ubuntu-server
Uptime: 5 days, 3 hours, 42 minutes
Platform: linux
Arch: x64
Username: ubuntu
Memory: 15.7 GB / 32 GB
CPUs: 8 cores
IP: 192.168.1.100
```

## Related Commands

- [status](./status.md) - Show overall node status
- [status services](./services.md) - Show all services status
- [doctor](../doctor/doctor.md) - Run diagnostics on the system
