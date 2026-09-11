# Linux self-hosted runner contract

The persistent Linux runners used by `kotlin-ci` and `rust-ci` are containerized for reproducible toolchains. The runner image is the versioned environment; jobs must not install host packages or modify host device permissions.

The runner container runs as the non-root `runner` user with default seccomp/AppArmor confinement, no added capabilities, and no `/var/run/docker.sock` mount. The Kotlin runner receives only `/dev/kvm` through the `kvm` supplementary group for the Android emulator. The host should expose that device as `root:kvm` with mode `0660`.

The image must include the build-essential, CMake, GMP/OpenSSL/PkgConfig libraries, GitHub CLI, protoc 32.0, JDK 17, Android SDK/NDK, Rust, and the Cargo tools required by the workflows. Rebuild and repin the image when those dependencies change.

Jobs that need to publish Docker images run on GitHub-hosted runners. A future self-hosted job that needs Docker must use a separately isolated runner; the `kotlin-ci` and `rust-ci` containers must not regain a host Docker socket.
