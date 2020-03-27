Contributing to Dash SDK
======================

First off, thanks for taking the time to contribute!

The following is a set of guidelines for contributing. These are mostly guidelines, not rules. Use your best judgment, and feel free to propose changes to this document in a pull request.

#### Table Of Contents

1. [Code of Conduct](#code-of-conduct)
1. [Styleguides](#styleguides)
  	+ [Code](#code)
  	+ [Conventional commits](#conventional-commits)
  	+ [Issues](#issues)
  	+ [Pull Requests](#pull-requests)


## Code of Conduct

This project and everyone participating in it is governed by the [Code of Conduct](CODE_OF_CONDUCT.md).   
By participating, you are expected to uphold this code. Please report unacceptable behavior to [alex@dash.org](mailto:alex@dash.org).

## Styleguides

#### Code

* Try to write your code following the style in the repo already.
* Comply with the standard being setup (example : ESLint, prettier,...)

#### Conventional Commits

A valid PR's commits and title are expected to comply with the conventional commits standard. The valid types are : 

- **feat** (Features): Used for a new feature being implemented
- **fix** (Bug Fixes): Used for bug fixes
- **impr** (Improvements): Used to describe an improvement to an existing feature (format can be description this)
- **docs** (Documentation): Changes happening on the documentation files
- **style**: Changes that do not affect the meaning of the code (white-space, formatting, missing semi-colons, etc)
- **refact** (Code Refactoring): A code change that neither fixes a bug nor adds a feature
- **perf** (Performance Improvements): A code change that improves performance
- **ci** (Continuous Integrations) Changes to our CI configuration files and scripts (example scopes: Travis, Circle, BrowserStack, SauceLabs)
- **chore**: Other changes that don't modify src or test files
- **revert**: Reverts a previous commit
- **test**: Adding missing tests or correcting existing tests
- **build**: Changes that affect the build system or external dependencies (example scopes: gulp, broccoli, npm)

Examples : 

- feat: added new Document parsing module
- impr(Document): parsing handle schema validation
- fix(Document): parsing leak memory fixed

Please remember that we depend on commit titles to follow changes and research previous modifications.   
Don't hesitate to use commit messages to explain more about the changes.   

#### Issues

* Demonstrating the issue by creating a JSFiddle (you can inherit Dash SDK from unpkg) is definitely welcome. 
* **Use a clear and descriptive title** for the issue to identify the suggestion.
* **Provide a comprehensive description of the suggested enhancement** in as much detail as possible. (a template is automatically generated for you when creating an issue / pr)
* (If applicable) **Provide specific examples to demonstrate the steps**.

#### Pull Requests

* **Use a clear and descriptive title** for the issue to identify the suggestion.
* Include any relevant issue numbers in the PR body, not the title.
* **Provide a comprehensive description of all changes made.**
