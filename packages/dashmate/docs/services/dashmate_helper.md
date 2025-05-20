## Dashmate CLI and Helper

Dashmate CLI and Helper are using to manage a Dash Platform node.

Both CLI and Heler using Docker to manage containers and services
defined in docker-compose files.

Dashmate helper serves JSON RPC HTTP API that replicates CLI commands.
It needs to be enabled with the `dashmate.helper.enabled` configuration option.
Helper also performs some background tasks such as ZeroSSL certificates renewal.

| Service                   | Port Purpose         | Default Value | Config Path                                  | Default Host Binding | Host Config Path |
|---------------------------|----------------------|---------------|----------------------------------------------|---------------------|-----------------|
| **Dashmate Helper**       | API                  | 9100          | `dashmate.helper.api.port`                   | 127.0.0.1 (local)   | (fixed)         |
