# Linux self-hosted runner contract

The persistent Linux runners used by `kotlin-ci` and `rust-ci` are containerized for reproducible toolchains. The runner image is the versioned environment; jobs must not install host packages or modify host device permissions.

The runner container runs as the non-root `runner` user with default seccomp/AppArmor confinement, no added capabilities, and no `/var/run/docker.sock` mount. The Kotlin runner receives only `/dev/kvm` through the `kvm` supplementary group for the Android emulator. The host should expose that device as `root:kvm` with mode `0660`.

The image must include build-essential, clang, llvm, libsnappy-dev, CMake, GMP/OpenSSL/PkgConfig libraries, GitHub CLI, protoc 32.0, JDK 17, Android SDK/NDK, Rust, cargo-llvm-cov 0.9.1, cargo-nextest 0.9.144, cargo-machete 0.9.2, cargo-ndk 4.1.2, and the other Cargo tools required by the workflows. The shared Rust action verifies clang, llvm, and libsnappy-dev rather than installing them. Rebuild and repin the image when those dependencies change.

The persistent macOS wallet runner follows the same no-suppressed-install rule and must provision cargo-machete 0.9.2 in its image. Jobs that need to publish Docker images, or the nightly hosted Kotlin emulator job, run on ephemeral GitHub-hosted runners; those workflows are an explicit exception and pin their cargo-ndk installation to 4.1.2. A future self-hosted job that needs Docker must use a separately isolated runner; the `kotlin-ci` and `rust-ci` containers must not regain a host Docker socket.
