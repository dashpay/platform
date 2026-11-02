# Repository documentation

Several of the packages in this repository contain developer documentation. This
folder is used to aggregate docs from several packages into one MkDocs site
that can be served locally (see below).

The published [GitHub Pages site](https://dashpay.github.io/platform/) is built
by [book.yml](/.github/workflows/book.yml) from the current dev branch. It
serves The Dash Platform Book, the architecture and conventions guide for the
Rust codebase (source under [`book/`](/book/)), at its root, with the generated
Rust, gRPC and JavaScript API references under `/api/`. Contributors should
start with the book's
[Coding Conventions](https://dashpay.github.io/platform/contributing/coding-conventions.html)
chapter (source: [`book/src/contributing/coding-conventions.md`](/book/src/contributing/coding-conventions.md)).

## Viewing documentation locally

You can use [MkDocs](https://www.mkdocs.org/getting-started/) to serve the
documents locally. From the root of the repository, do the following:

- Run [`./scripts/prepare_docs.sh`](/scripts/prepare_docs.sh)
- Run `mkdocs serve`
- Open the returned URL (typically http://127.0.0.1:8000/)
