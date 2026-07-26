# Validating an Envoy version bump for the Gateway

The gateway runs Envoy with a config rendered from
[`templates/platform/gateway/envoy.yaml.dot`](../../templates/platform/gateway/envoy.yaml.dot),
and the image is pinned in [`configs/defaults/getBaseConfigFactory.js`](../../configs/defaults/getBaseConfigFactory.js)
(`platform.gateway.docker.image`). `dashpay/envoy:<version>-impr.N` is a stock
`envoyproxy/envoy:v<version>` plus Envoy's hot-restart supervisor, so a stock release can be
validated before the custom image exists.

Bumping that pin means the rendered config has to keep loading, and the parts of the gateway that
only exist at runtime — the rate limit service wire protocol and the grpc-web over-limit reply —
have to keep working. `scripts/validate-envoy-config.js` checks both.

## Running the harness

```bash
cd packages/dashmate

# validate against the currently pinned version
yarn node scripts/validate-envoy-config.js

# validate against a candidate, side by side with the pin
yarn node scripts/validate-envoy-config.js v1.35.11 v1.39.0

# add the runtime checks (starts Envoy + the rate limiter + redis on a throwaway network)
yarn node scripts/validate-envoy-config.js --smoke v1.39.0

# the custom image docker-envoy publishes, which is what dashmate actually runs
yarn node scripts/validate-envoy-config.js --smoke dashpay/envoy:1.39.0-impr.1

yarn node scripts/validate-envoy-config.js --list        # the variant matrix
yarn node scripts/validate-envoy-config.js --out=/tmp/x  # keep renderings and logs
```

Requires `docker`, `curl` and `openssl`. Bare tags resolve against `envoyproxy/envoy`; anything
containing `:` is used as a full image reference. With no argument the baseline is derived from the
pinned image, so the default run always targets what dashmate currently ships.

### Config validation

The template branches on the TLS provider, the rate limiter, metrics/admin and the access-log
settings, so the harness renders one config per branch (`--list` prints them) and runs Envoy's own
validator on each:

```
docker run --rm --entrypoint envoy -v <rendered>:/etc/envoy/envoy.yaml:ro <image> \
  --mode validate -c /etc/envoy/envoy.yaml
```

`--mode validate` builds the entire config graph — listeners, filter chains, TLS contexts,
clusters, the overload manager — and exits. Unknown or removed fields fail hard; fields that are
merely deprecated log `Deprecated field: … Using deprecated option '<field>'`, which the harness
reports per variant. Both paths are worth trusting only because they were confirmed to fire: an
injected bogus field fails validation, and a config using a known-deprecated field
(`UpstreamTlsContext.enforce_rsa_key_usage`) produces the warning the harness greps for.

### Runtime checks (`--smoke`)

Boots redis, `platform.gateway.rateLimiter.docker.image` and the Envoy image under test on a
throwaway Docker network with the rendered default config, then asserts:

1. Envoy reaches `starting workers` and logs no warnings — this also covers `STRICT_DNS`
   resolution of the upstream service names.
2. Exactly `platform.gateway.rateLimiter.requestsPerUnit` requests pass, and the next one is
   rejected — i.e. Envoy's RLS v3 calls to the pinned rate limiter build are understood.
3. The grpc-web over-limit reply is `HTTP 200` + `grpc-status: 8` + `ratelimit-reset`, which is what
   browser clients need for node backoff.
4. The native gRPC over-limit reply is `HTTP 200` + `grpc-status: 8`.

The upstreams are stand-ins, so pre-limit requests answer `503`. The rate limit decision happens
before routing, which is what these checks are about.

### What it does not cover

Behavior changes that a valid config cannot reveal (see the release-note review below), throughput
and memory under real load, the hot-restart supervisor in the custom image, and anything requiring
real upstreams — run `yarn test:suite` against a node for that.

## Assessment: v1.35.11 → v1.39.0 (July 2026)

This is the bump the pin now carries — `dashpay/envoy:1.39.0-impr.1`.

Verdict: **go**, no config edits required. 8/8 variants validate clean and all runtime checks pass
on `envoyproxy/envoy:v1.39.0` and on the custom `dashpay/envoy:1.39.0-impr.1` (both its amd64 and
arm64 manifests), with results identical to the `v1.35.11` baseline (which was also run
through `dashpay/envoy:1.35.11-impr.1`). No field in the rendered config is
deprecated or removed at v1.39.0, and nothing in the `1.36.0`–`1.39.0` release notes removes config
we use. `envoyproxy/ratelimit:3fcc3609` (April 2024) still interoperates: the filter's
`transport_api_version: V3` remains current, and the over-limit path was exercised end to end.

Two behavior changes do alter how the gateway behaves on this version, without changing the config:

- **The overload actions keyed on `global_downstream_max_connections` start working.** v1.37.0 fixed
  that monitor to actually trigger actions ("previously, actions never triggered"). So
  `disable_http_keepalive` at 95% and `stop_accepting_connections` at 100% of
  `platform.gateway.maxConnections` become live: at ~950 of 1000 connections Envoy will start
  draining HTTP/2 connections with GOAWAY, which clients see as connection churn. The
  `overload.envoy.resource_monitors.global_downstream_max_connections.pressure` gauge is new and
  confirms it (absent on v1.35.11, present on v1.39.0). Consider whether the shipped default is the
  right threshold now that it is enforced.
- **HTTP/2 flood protection now scales with active streams.** v1.39.0 closed a bypass where
  PRIORITY/WINDOW_UPDATE flood protection could be evaded by rapid stream churn; frame budget is now
  tied to active streams. The gateway sets small windows (64 KiB per stream) and
  `max_concurrent_streams` of 10, so long-lived streaming endpoints emit many WINDOW_UPDATE frames.
  Watch `http.ingress_http.http2.inbound_window_update_frames_flood` after the bump; the change can
  be reverted with the `envoy.reloadable_features.http2_flood_protection_active_streams` runtime
  guard.

Lower-risk notes, all reversible via runtime guards, none needing config changes: upstream
`max_concurrent_streams` now defaults to 1024 instead of 2^31-1 (v1.36.0) — far above the
`max_requests: 100` circuit breakers; the TLS inspector used by the self-signed provider now
enforces client TLS 1.0–1.3 (v1.39.0); upstream transport failure reasons no longer reach response
bodies (v1.39.0); `HeaderMatcher` matches multi-value headers individually (v1.39.0) — the only
header matcher in the config is a `present_match` on `x-grpc-web`, which is unaffected; and a
`timeout: 0s` on the rate limit filter now means "no timeout" (v1.38.0) — the config sets `5s`.

v1.39.0 also adds `Http2ProtocolOptions.stream_reset_burst`/`stream_reset_rate`, worth considering
separately as rapid-reset hardening.

### Notes for the bump

- The hot-restart supervisor in the custom image is outside what the harness covers, because it
  overrides the entrypoint to reach Envoy's flags. It was checked by hand on
  `dashpay/envoy:1.39.0-impr.1`: the container boots through `hot-restarter.py` at epoch 0, `SIGHUP`
  forks a child at epoch 1 and the parent exits cleanly, which is the zero-downtime path dashmate
  uses after certificate renewal. Worth repeating on future bumps.
- v1.39.0 is the only 1.39 release; the HTTP/2 flood-protection fix was not backported to the
  1.35–1.38 patch lines. The pin is also two patches behind on its own line — v1.35.13 carries the
  June 2026 CVE batch, none of which touch the extensions this gateway loads.
- Unrelated pre-existing finding, identical on both versions: the route returning `grpc-status: 12`
  for unsupported API versions never matches. `RouteMatch.safe_regex` must match the whole path, and
  the pattern `\/org\.dash\.platform\.dapi\.v[1-9]+\.` only covers a prefix, so
  `/org.dash.platform.dapi.v1.Platform/getIdentity` gets a plain `404`.
