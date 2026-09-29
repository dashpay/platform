# Coexisting ordinary AMD64 runner images

The selector retains the existing generic ordinary labels only for the exact
already-provisioned AMD64 contract `d272d01bcf3dfa620bbab1e9f31c3e1987862d33ec876bbad83c80f29de41a58`.
For any different AMD64 manifest (including a recipe-only change), branch pushes
and PRs that do not change that manifest require Linux/X64 plus:

`platform-image-manifest-<canonical-full-manifest-sha256>-<rust|kotlin|npm>`

This is an ordinary pool label, not a candidate label or an admission token.
Runtime `ci-image-contract verify` remains mandatory. PRs changing AMD64
requirements still need the exact current-head published candidate and its
unchanged trust gates. Explicit ARM64 validation and its provisioning contract
remain unchanged; new AMD64 ordinary contracts deliberately use X64 capacity.

## Safe rollout order

1. Keep legacy runner registrations/images/labels available for older branches
   and existing PR heads. Do not overwrite an active runner or change its image.
2. Obtain the exact published immutable new image from a successful trusted
   publication. Verify manifest/recipe/provenance, confinement and applicable
   compiler/KVM checks before creating separate capacity with fresh registration
   and work volumes. Preserve the existing repository/group scope and resource
   reserve/cleanup safeguards; do not copy Gateway credentials to the host.
3. Give that new capacity only the corresponding versioned kind label, never
   `rust-ci`, `kotlin-ci`, `npm-pr`, or any `platform-image-pr-*` candidate label.
   Do not label an image merely because its tool versions look similar.
4. Merge the selector repair normally before a new manifest becomes the branch
   default. Verify actual candidate jobs and new ordinary capacity before merging
   the recipe PR; source tests or a queued runner are not execution proof.
5. Observe real ordinary exact-contract jobs and preserve old capacity until all
   remaining consuming branches/heads are migrated or no longer need it. Rollback
   must preserve both contracts; never relabel an incompatible image to clear CI.

This source change does not create runners or register/alter any label, image,
permission, workload, candidate allocator, or cleanup timer. Missing compatible
new capacity intentionally queues work rather than selecting an old image.
