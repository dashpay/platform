# Infra runbook: the `maven-central` GitHub Environment

This is the **infrastructure/admin half** of enabling Maven Central publishing for
`org.dashj:dash-sdk-android`. The **code half** — the release workflow that gates publishing
behind this environment — is [dashpay/platform#4193](https://github.com/dashpay/platform/pull/4193)
(`.github/workflows/kotlin-sdk-release.yml`). Neither half is complete on its own: the workflow
already restricts every publishing secret to the `environment: maven-central` job, but that boundary
is only airtight once the secrets are actually **scoped to this environment** and the
repository/organization-scoped copies are removed (step 3 below).

Developer-facing context on the release flow itself is in
[`PUBLISHING.md`](./PUBLISHING.md); this file is the checklist for whoever has org-owner /
repo-admin rights on `dashpay/platform`.

All steps are in **GitHub → `dashpay/platform` → Settings → Environments**. Developers cannot
create environments or environment secrets — this requires an admin.

---

## 1. Create the environment

Settings → Environments → **New environment** → name it exactly **`maven-central`**
(lowercase, hyphen). It must match the `environment:` value in `kotlin-sdk-release.yml` verbatim —
any mismatch means the deploy job runs **without** the gate.

## 2. Add the approval gate

- Enable **Required reviewers** and add the person/team allowed to approve a Maven Central publish
  (e.g. the release owners). One reviewer is sufficient; two is fine. This is what makes the
  `maven-central-deploy` job pause for human approval before it can publish irrevocably.
- Recommended: **Deployment branch and tag policy → Selected branches and tags → Add rule → Tag →
  `kotlin-sdk-v*`**. This restricts the environment (and therefore the credentials) to real
  Kotlin-SDK release tags only.
- Optional: a short **Wait timer** for a cooling-off window. Not required.

## 3. Scope the publishing secrets to the environment (and delete the repo/org copies)

Add each of the following as an **Environment secret on `maven-central`**
(Environment → Environment secrets → Add secret), using the same values currently stored at the
repository/organization level:

| Secret name | What it is |
|---|---|
| `JRELEASER_MAVENCENTRAL_SONATYPE_USERNAME` | Central Portal / Sonatype username (or token username) |
| `JRELEASER_MAVENCENTRAL_SONATYPE_PASSWORD` | Central Portal / Sonatype password (or token) |
| `JRELEASER_GPG_SECRET_KEY` | ASCII-armored GPG **private** signing key |
| `JRELEASER_GPG_PASSPHRASE` | passphrase for that GPG key |
| `JRELEASER_GPG_PUBLIC_KEY` | GPG public key (not sensitive, but keep it alongside the others) |

Then **delete the repository-scoped copies and remove `dashpay/platform`'s access to any
organization-scoped copies of these same five names** (Settings → Secrets and variables → Actions,
and the organization secrets list). Delete an organization secret entirely only if no other
repository uses it; otherwise update its repository access policy to exclude `dashpay/platform`.

> **Removing this repository's access is the crux.** As long as a repository-scoped copy exists, or
> an organization-scoped copy remains available to `dashpay/platform`, an unprotected job in this
> repository can still read the credentials and the security finding that motivated #4193 is not
> actually closed. After this step, only the approved `maven-central` job can see them.
>
> `JRELEASER_GPG_PUBLIC_KEY` is a public key and not itself sensitive, but move it too so all five
> live in one place and the build job has no reason to touch the environment.

## 4. Verify

1. Push a real `kotlin-sdk-vX.Y.Z` tag. To verify with `workflow_dispatch`, dispatch the
   workflow **at that tag ref**, for example:
   `gh workflow run kotlin-sdk-release.yml --ref kotlin-sdk-vX.Y.Z -f tag=kotlin-sdk-vX.Y.Z`.
   Supplying only the `tag` input while dispatching from a branch does not satisfy a tag-only
   environment policy. The run should **pause** at the `maven-central-deploy` job showing
   *"Waiting for review."*
2. It proceeds to publish only after an approved reviewer clicks **Approve and deploy**.
3. Confirm the `build-and-release` job has **no** access to the five secrets — after #4193 it does
   not reference them at all; its only context token is the auto-provided `GITHUB_TOKEN`.

> **Transitional note — a pre-migration approval is NOT a safe no-op.** The secret check runs only
> *after* environment approval, in the gated `maven-central-deploy` job. It is tempting to assume
> that, before step 3, that job sees no secrets and simply no-ops — but GitHub's secret precedence
> does not confine an environment job to environment-scoped secrets. An environment secret only
> *overrides* a same-named repository/organization secret; when the environment secret is **absent**,
> an accessible repository- or organization-scoped copy still resolves through `${{ secrets.NAME }}`.
> Section 3 notes these five credentials currently exist at those broader scopes, so until step 3
> revokes this repository's access, they remain readable in the gated job. Approving a valid
> `kotlin-sdk-v*` release tag can therefore pass the all-or-nothing check and perform an
> **irrevocable Maven Central publish** — not a no-op test. Complete step 3 **before** approving any
> verification run. Only when all five secrets are genuinely unavailable to this repository does the
> gated job no-op; a partially available set still hard-fails (that guard is preserved, just
> relocated behind the gate).

---

### One-line summary for the ticket

> On `dashpay/platform`, create a `maven-central` GitHub Environment with required reviewers (and a
> `kotlin-sdk-v*` tag policy), add the five `JRELEASER_*` publishing secrets as **environment**
> secrets, and **delete the repository-scoped copies and revoke this repo's access to any
> organization-scoped copies** of those same secrets (delete an org secret outright only if no
> other repository uses it). Code
> side: dashpay/platform#4193.
