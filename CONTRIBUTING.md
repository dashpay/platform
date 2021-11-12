Contributing to Dash Platform
============================

The Dash Platform project operates an open contributor model where anyone is
welcome to contribute towards development in the form of peer review, testing
and patches. This document explains the practical process and guidelines for
contributing.

Branches, bugfixes and new features
-----------------------------------

Current stable release is in the `master` branch. This branch is meant to be stable, so the
only PRs that are made to master should be bugfixes. The development of the next release 
happens in the `vX-dev` branch, where `X` is the next version number that's going to be released.
All new features PRs must be made to the current dev branch.

The body of the PR also should be following the default PR template that will appear one you
open a PR through GitHub. Please fill all template field with enough description about what the
patch does together with any justification/reasoning. You should include references to any
discussions (for example other tickets or mailing list discussions).

If a pull request is not to be considered for merging (yet), please set its status as "Draft" in
GitHub.

Conventional commits
--------------------

All commits and pull request titles should follow the conventional commits scheme.
PR titles follow the `<type>(optional scope): <description> scheme`. Please check the link above
to see valid types. When making a change to a specific component, please specify the name of
the component inside scope. So, for example, if you're developing a new feature for the js-sdk,
the PR title should look like this: `js-sdk(feat): new amazing feature`.

For more details on allowed types and more info about conventional commits, please check the 
[conventional commits docs](https://www.conventionalcommits.org/en/v1.0.0/). For available scopes
please check [.github/semantic.yml](.github/semantic.yml) file.

In general [commits should be atomic](https://en.wikipedia.org/wiki/Atomic_commit#Atomic_commit_convention)
and diffs should be easy to read. For this reason do not mix any formatting
fixes or code moves with actual code changes.

Code conventions
----------------

Please ensure that the code you wrote adheres to the code style adopted in the project - AirBnB 
style for JS code, and that all linting check are passing.

Testing
-------

The code must be acompanied by the tests that check the functionality. Test for individual
components are stored inside `packages/<component_name>/tests`, and for e2e test inside
`packages/platform-test-suite`.

Test case name should start with a lowercase "should", i.e. "should do x".
Unit and integration tests should mirror the file structure of `/src` or `/lib` (depending
on the component).

Code generally should be covered with unit and integration tests, and for larger chucks of
functionality functional or e2e tests should be written (when appropriate). Unit and integration
test should not make any network calls, and unit tests should mock all of its dependencies.

Squashing Commits
-----------------

If your pull request is accepted for merging, you may be asked by a maintainer
to squash and or [rebase](https://git-scm.com/docs/git-rebase) your commits
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

Copyright
---------

By contributing to this repository, you agree to license your work under the
MIT license unless specified otherwise at the top of the file itself. 
Any work contributed where you are not the original author must contain its 
license header with the original author(s) and source.
