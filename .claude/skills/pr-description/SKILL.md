---
name: pr-description
description: Generate a pull request description for the current branch. Use when the user asks to create a PR description, write PR body, or prepare a PR for review.
argument-hint: "[base-branch]"
---

# Generate PR Description

Generate a pull request title and description for the current branch using the project's standard template.

## Steps

1. Determine the base branch:
   - Use the argument if provided
   - Otherwise, auto-detect: `git remote set-head origin --auto >/dev/null 2>&1 && git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's|refs/remotes/||'`
   - Fall back to `v3.1-dev` if the command above fails

2. Gather context by running these git commands:
   - `git log --oneline $(git merge-base HEAD <base>)..HEAD` — all commits on this branch
   - `git diff <base>...HEAD --stat` — files changed summary
   - `git diff <base>...HEAD` — full diff (read selectively for large diffs)

3. Analyze the changes to understand:
   - What problem is being solved or feature added
   - What specific code changes were made
   - Whether there are breaking changes
   - What tests were added or modified

4. Output a suggested PR title using conventional commits format:
   - Scopes: `sdk`, `drive`, `dpp`, `dapi`, `dashmate`, `wasm-dpp`, `wasm-sdk`, `platform`
   - Types: `feat`, `fix`, `refactor`, `chore`, `docs`, `test`, `build`
   - Add `!` after the type for breaking changes (e.g. `feat!:`)
   - Format: **Suggested title:** `type(scope): description`

5. Fill in this PR template (preserve all HTML comments exactly as shown):

```markdown
<!--- Provide a general summary of your changes in the Title above -->
<!--- Pull request titles must use the [conventional commits](https://www.conventionalcommits.org/en/v1.0.0/#summary) format -->

## Issue being fixed or feature implemented
<!--- Why is this change required? What problem does it solve? -->
<!--- If it fixes an open issue, please link to the issue here. -->

<Explain WHY this change is needed. Link to GitHub issues if applicable.>

## What was done?
<!--- Describe your changes in detail -->

<Describe the changes in detail. Use bullet points for multiple changes. Reference actual file paths, types, and function names.>

## How Has This Been Tested?
<!--- Please describe in detail how you tested your changes. -->
<!--- Include details of your testing environment, and the tests you ran to -->
<!--- see how your change affects other areas of the code, etc. -->

<List specific test files added/modified. Mention `cargo test` or `yarn test` commands. Describe manual testing if applicable.>

## Breaking Changes
<!--- Please describe any breaking changes your code introduces and verify that -->
<!--- the title includes "!" following the conventional commit type (e.g. "feat!: ..."-->

<Describe any breaking changes, or "None".>

## Checklist:
<!--- Go over all the following points, and put an `x` in all the boxes that apply. -->
- [ ] I have performed a self-review of my own code
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] I have added or updated relevant unit/integration/functional/e2e tests
- [ ] I have added "!" to the title and described breaking changes in the corresponding section if my code contains any
- [ ] I have made corresponding changes to the documentation if needed

**For repository code-owners and collaborators only**
- [ ] I have assigned this pull request to a milestone
```

## Output Format

Output the entire PR description (title + body) as a single raw Markdown code block so the user can easily copy and paste it. Wrap the output in triple backticks with the `Markdown` language tag.

## Guidelines

- Keep the description **concise** — avoid walls of text. Prefer short bullet points over paragraphs
- Be specific — reference file paths, struct/function names, and types
- For "How Has This Been Tested?", check `git diff` for new `*test*`, `*spec*` files. Briefly describe what tests cover (1 line per test file), not every individual test case
- For large diffs, focus on architectural changes rather than listing every file
- Group related changes together in "What was done?" using sub-headers if needed
- Avoid repeating the same information across sections
