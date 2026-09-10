# Repository documentation

Several of the packages in this repository contain developer documentation. This
folder is used to aggregate docs from several packages and then produce a
consolidated [GitHub Pages site](https://dashpay.github.io/platform/) using
MkDocs. The GitHub workflow described in [docs.yml](/.github/workflows/docs.yml)
builds the documents and publishes them.

The architecture and conventions guide for the Rust codebase is a separate
mdBook under [`book/`](/book/), published by
[book.yml](/.github/workflows/book.yml). Contributors should start with its
[Coding Conventions](/book/src/contributing/coding-conventions.md) chapter.

## Viewing documentation locally

You can use [MkDocs](https://www.mkdocs.org/getting-started/) to serve the
documents locally. From the root of the repository, do the following:

- Run [`./scripts/prepare_docs.sh`](/scripts/prepare_docs.sh)
- Run `mkdocs serve`
- Open the returned URL (typically http://127.0.0.1:8000/)
