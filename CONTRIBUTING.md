Contributing to Dash Platform
============================

The Dash Platform project operates an open contributor model where anyone is
welcome to contribute towards development in the form of peer review, testing
and patches. This document explains the practical process and guidelines for
contributing.

Branch logic (master stable, dev branches)
PR (atomic (dashd), Convential commit, squash)
AC for PRs:
Testing https://docs.google.com/document/d/1DhXios0fvK93LvNXr-WIf9C9kreYsTF5MOLwnJlhkzU/edit
Chekcs
Release process

Branches, bugfixes and new features
--------

Current stable release is in the `master` branch. This branch is meant to be stable, so the
only PRs that are made to master should be bugfixes. The development of the next release 
happens in the `vX-dev` branch, where `X` is the next version number that's going to be released.
All new features PRs must be made to the current dev branch.

### Conventional commits

All commits and pull request titles should follow the conventional commits scheme.
PR titles follow the `<type>(optional scope): <description> scheme`. Please check the link above
to see valid types. When making a change to a specific component, please specify the name of
the component inside scope. So, for example, if you're developing a new feature for the js-sdk,
the PR title should look like this: `js-sdk(feat): new amazing feature`.

For more details on allowed types and more info about conventional commits, please check the 
[conventional commits docs](https://www.conventionalcommits.org/en/v1.0.0/)


### Workflow

If a pull request is not to be considered for merging (yet), please set its status as "Draft" in
GitHub.

The body of the PR also should be following the default PR template that will appear one you
open a PR through GitHub. Please fill all template field with enough description about what the
patch does together with any justification/reasoning. You should include references to any 
discussions (for example other tickets or mailing list discussions).


Contributor Workflow
--------------------

The codebase is maintained using the "contributor workflow" where everyone
without exception contributes patch proposals using "pull requests". This
facilitates social contribution, easy testing and peer review.

To contribute a patch, the workflow is as follows:

1. Fork repository
1. Create topic branch
1. Commit patches

The project coding conventions in the [developer notes](doc/developer-notes.md)
must be adhered to.

In general [commits should be atomic](https://en.wikipedia.org/wiki/Atomic_commit#Atomic_commit_convention)
and diffs should be easy to read. For this reason do not mix any formatting
fixes or code moves with actual code changes.

Commit messages should be verbose by default consisting of a short subject line
(50 chars max), a blank line and detailed explanatory text as separate
paragraph(s), unless the title alone is self-explanatory (like "Corrected typo
in init.cpp") in which case a single title line is sufficient. Commit messages should be
helpful to people reading your code in the future, so explain the reasoning for
your decisions. Further explanation [here](http://chris.beams.io/posts/git-commit/).

If a particular commit references another issue, please add the reference. For
example: `refs #1234` or `fixes #4321`. Using the `fixes` or `closes` keywords
will cause the corresponding issue to be closed when the pull request is merged.

- Push changes to your fork
- Create pull request

Squashing Commits
---------------------------
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

The following applies to code changes to the Dash Platform project (and related
projects such as dashcore-lib), and is not to be confused with overall Dash
Network Protocol consensus changes.

Whether a pull request is merged into Dash Platform rests with the project merge
maintainers.

Maintainers will take into consideration if a patch is in line with the general
principles of the project; meets the minimum standards for inclusion; and will
judge the general consensus of contributors.

In general, all pull requests must:

- Have a clear use case, fix a demonstrable bug or serve the greater good of
  the project (for example refactoring for modularisation);
- Be well peer reviewed;
- Have unit tests and functional tests where appropriate;
- Follow code style guidelines ([C++](doc/developer-notes.md), [functional tests](test/functional/README.md));
- Not break the existing test suite;
- Where bugs are fixed, where possible, there should be unit tests
  demonstrating the bug and also proving the fix. This helps prevent regression.

Patches that change the consensus rules are considerably more involved than
normal because they affect the entire ecosystem and so must have a numbered DIP. 
While each case will be different, one should be prepared to expend more time and effort than for
other kinds of patches because of increased peer review and consensus building
requirements.


### Peer Review

Anyone may participate in peer review which is expressed by comments in the pull
request. Typically reviewers will review the code for obvious errors, as well as
test out the patch set and opine on the technical merits of the patch. Project
maintainers take into account the peer review when determining if there is
consensus to merge a pull request (remember that discussions may have been
spread out over GitHub, mailing list and IRC discussions). The following
language is used within pull-request comments:

Project maintainers reserve the right to weigh the opinions of peer reviewers
using common sense judgement.

Where a patch set affects consensus critical code, the bar will be set much
higher in terms of discussion and peer review requirements, keeping in mind that
mistakes could be very costly to the wider community. This includes refactoring
of consensus critical code.

Where a patch set proposes to change the consensus, it must have been
discussed extensively, be accompanied by a widely
discussed DIP and have a generally widely perceived technical consensus of being
a worthwhile change based on the judgement of the maintainers.

#### Verifying a Rebase

When someone rebases their PR, it can often be very difficult to ensure that
extra changes were not included in that force push. This changes could be anything
from merge conflicts to someone attempting to sneak something into the PR. To check
that a PR is the same before and after force push, you can use the following function.
Place this function in your `~/.bashrc`. In order for this function to work, both the
before and after commits must be present locally.

```
function gfd() {
        local fp1=$(git merge-base --fork-point develop $1)
        local fp2=$(git merge-base --fork-point develop $2)
        echo fp1=$fp1
        echo fp2=$fp2
        diff --color=always -u -I'^[^-+]' <(git diff $fp1..$1) <(git diff $fp2..$2)
}
```

Copyright
---------

By contributing to this repository, you agree to license your work under the
MIT license unless specified otherwise at the top of the file itself. 
Any work contributed where you are not the original author must contain its 
license header with the original author(s) and source.
