# Dash Platform Docker Development tips

## Introduction

This document describes some tips and optimizations needed to speed up build of Docker images.
As Docker images are intensively used as part of the Dashmate workflow, these tips will be mainly
useful to:

* developers, building Platfrom directly from git sources,
* Github Actions developers, trying to optimize caching of Docker caching.

When building Dash Platform's Docker images, you can encounter the following issues:

1. Docker build without proper caching is relatively slow due to need to rebuild all sources from the scratch
2. Building for other architectures using emulation (eg. running ARM64 VM on a x86-64 host) is very slow
3. Cross-compiling Platform can be tricky due to some dependency issues (for example, cross-compilation of
   librocksdb-sys is hard)
4. With intensive caching, cache can grow very fast and usemore than 10 GB, which is above default cache size limit in
   Docker buildx and Github Actions.

## Caches

We implement the following levels of caching:

1. Docker caching: layers and cache mounts (`RUN --mount=type=cache,...`)
2. `sccache`

### Docker-level caching cache mounts

Cache mounts include some cachable elements of RUST_HOME (including downloaded dependencies), as well as
[Cargo target dir](https://doc.rust-lang.org/cargo/guide/build-cache.html). As the target dir grows to multiple
gigabytes, Docker cache garbage collector must be tuned accordingly.

For example, you can create the following Buildkit config file in `$HOME/.config/buildkit/buildkitd.toml`:

```toml
[worker.oci]
gc = true
gckeepstorage = 40000 # 40 GB

[[worker.oci.gcpolicy]]
all = true
keepBytes = 30000000000 # 30 GB
keepDuration = 432000 # 5 days
```

and create buildx builder instance:

```bash
docker buildx create --config "$HOME/.config/buildkit/buildkitd.toml" --name local --use --bootstrap
```

Double-check that configured garbage collection policy was correctly applied:

```bash
docker exec -ti buildx_buildkit_local0  buildctl debug workers -v
```

```plain
GC Policy rule#0:
    All:  true
    Keep Duration: 120h0m0s
    Keep Bytes: 30GB
```

### Sccache

[Sccache](https://github.com/mozilla/sccache) is an additional layer of caching that can be used to improve build speed. Sccache can cache build artifacts using multiple storage systems, like local filesystem, Github Actions, S3 buckets, memcache, and others.

For local development in Docker, sccache uses Docker cache mounts. Alternative solution can be to use memcached daemon for caching.

Sccache is disabled by default.

#### Enabling sccache

To enable sccache, set `RUSTC_WRAPPER=sccache` build argument when building the Docker image. For example:

```bash
docker buildx build --build-arg RUSTC_WRAPPER=sccache --target drive-abci .
```

#### Sccache and memcached

To use sccache with memcache, setup local memcached daemon with reasonable amount of RAM (4GB here):

```bash
docker run --name memcache -d -p 172.17.0.1:11211:11211 memcached memcached -m 4096 -l 0.0.0.0
docker update --restart unless-stopped memcache 
```

Replace `172.17.0.1` with IP address of the host running memcached accessible from your Docker containers (and not accessible from external networks).

##### Using memcache with Dashmate

Before starting dashmate build, set `SCCACHE_MEMCACHED` to address of your memcache server:

```bash
export RUSTC_WRAPPER=sccache
export SCCACHE_MEMCACHED=tcp://172.17.0.1:11211
```

##### Using memcache with buildx

Pass SCCACHE_MEMCACHED build argument to Docker Buildx

```bash
docker buildx build -f Dockerfile --build-arg SCCACHE_MEMCACHED=tcp://[your.ip.address.here]:11211 --progress=plain --target drive-abci .
```

Replace `172.17.0.1` with IP address of the host running memcached accessible from your Docker containers.

### Using sccache in your IDE

You can also use the same cache for your local builds (eg. in your IDE). To do this, install sccache, and set `RUSTC_WRAPPER` environment variable before starting your IDE (on Linux, you can put them into `$HOME/.profile`):

```bash
export RUSTC_WRAPPER=/usr/local/bin/sccache
```

If you want your local system to use same cache as docker builds, set `SCCACHE_MEMCACHED`:

```bash
export SCCACHE_MEMCACHED=tcp://172.17.0.1:11211
```

If you use **Microsoft Visual Studio Code** and **rust-analyzer**, you can add forementioned environment variables to `"rust-analyzer.cargo.extraEnv` in your workspace config:

```json
"settings": {
    "rust-analyzer.rustfmt.extraArgs": [
    "+nightly"
    ],
  "rust-analyzer.cargo.buildScripts.useRustcWrapper": true,
  "rust-analyzer.cargo.extraEnv": {
   "SCCACHE_MEMCACHED": "tcp://192.168.1.91:11211",
   "CARGO_TARGET_DIR": "/home/lklimek/tmp/rust/target"
  }
}
```
