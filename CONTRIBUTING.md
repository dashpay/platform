Contributing to Dash Platform
=============================

The Dash Platform project operates an open contributor model where anyone is
welcome to contribute towards development in the form of peer review, testing
and patches. This document explains the practical process and guidelines for
contributing.


Branches, Bugfixes and New Features
-----------------------------------

The current stable release is in the `master` branch. This branch is meant to be stable, so the
only PRs made to master should be bugfixes. Development of the next release happens
on the `vX-dev` branch, where `X` is the next version number to be released.
All new feature PRs must be made to the current dev branch.

The body of the PR should also follow the default PR template that appears when you
open a PR on GitHub. Please fill all template fields with a sufficient description of what the
patch does, together with any justification/reasoning. You should include references to any
discussions (for example other tickets or mailing list discussions).

If a pull request is not (yet) ready to be considered for merging, please set its status to "Draft" on
GitHub.


Conventional Commits
--------------------

All commits and pull request titles should follow the Conventional Commits specification.
PR titles follow the `<type>(optional scope): <description>` scheme. Please see the specification linked below
for valid types. When making a change to a specific component, please specify the name of
the component inside the scope. For example, if you are developing a new feature for the SDK,
the PR title should look like this: `feat(sdk): amazing new feature`.

For more details on allowed types and more information about Conventional Commits, please see the 
[Conventional Commits specification](https://www.conventionalcommits.org/en/v1.0.0/). For available scopes,
please see the [.github/semantic.yml](.github/semantic.yml) file.

In general, [commits should be atomic](https://en.wikipedia.org/wiki/Atomic_commit#Atomic_commit_convention)
and diffs should be easy to read. For this reason, do not mix any formatting
fixes or code movement with actual code changes.


Code Conventions
----------------

Please ensure that the code you write adheres to the code style adopted in the project, and that all linting checks are passing. We use [AirBnB 
style](https://github.com/airbnb/javascript) for JS code.


Testing
-------

The code must be accompanied by tests to check the functionality. Tests for individual
components are stored inside `packages/<component_name>/tests`, while e2e test are inside
`packages/platform-test-suite`.

Test case names should start with a lowercase "should", i.e. "should do x".
Unit and integration tests should mirror the file structure of `/src` or `/lib` (depending
on the component).

Code should generally be covered by unit and integration tests, and functional or e2e tests should be written for larger chunks of
functionality (when appropriate). Unit and integration
tests should not make any network calls, and unit tests should mock all dependencies.


Squashing Commits
-----------------

If your pull request is accepted for merging, you may be asked by a maintainer
to squash and/or [rebase](https://git-scm.com/docs/git-rebase) your commits
before it will be merged. The basic squashing workflow is shown below.

    git checkout your_branch_name
    git rebase -i HEAD~n
    # n is normally the number of commits in the pull request.
    # Set commits (except the one in the first line) from 'pick' to 'squash', save and quit.
    # On the next screen, edit/refine commit messages.
    # Save and quit.
    git push -f # (force push to GitHub)

If you have problems with squashing (or other workflows with `git`), you can
alternatively enable "Allow edits from maintainers" in the right GitHub
sidebar and ask for help in the pull request.

Please refrain from creating several pull requests for the same change.
Use the pull request that is already open (or was created earlier) to amend
changes. This preserves the discussion and review that happened earlier for
the respective change set.

The length of time required for peer review is unpredictable and will vary from
pull request to pull request.


Pull Request Philosophy
-----------------------

Patchsets should always be focused. For example, a pull request could add a
feature, fix a bug, or refactor code; but not a mixture. Please also avoid super
pull requests which attempt to do too much, are overly large, or overly complex
as this makes review difficult.


"Decision Making" Process
-------------------------

Whether a pull request is merged into Dash Platform rests with the project merge
maintainers.

Maintainers will take into consideration if a patch is in line with the general
principles of the project and meets the minimum standards for inclusion.

In general, all pull requests must:

- Have a clear use case, fix a demonstrable bug or serve the greater good of
  the project (for example refactoring for modularisation);
- Have unit tests and functional tests where appropriate;
- Follow code style guidelines;
- Not break the existing test suite;
- Where bugs are fixed, where possible, there should be unit tests
  demonstrating the bug and also proving the fix. This helps prevent regression.


Release process
---------------

Coming soon.

Copyright
---------

By contributing to this repository, you agree to license your work under the
MIT license unless specified otherwise at the top of the file itself. 
Any work contributed where you are not the original author must contain its 
license header with the original author(s) and source.
