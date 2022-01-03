# Repository documentation

Several of the packages in this repository contain developer documentation. This
folder is used to aggregate docs from several packages and then produce a
consolidated [GitHub Pages site](https://dashevo.github.io/platform/) using
MkDocs. The GitHub workflow described in [docs.yml](/.github/workflows/docs.yml)
builds the documents and publishes them.

## Viewing documentation locally

You can use [MkDocs](https://www.mkdocs.org/getting-started/) to serve the
documents locally. From the root of the repository, do the following:

- Run [`./scripts/prepare_docs.sh`](/scripts/prepare_docs.sh)
- Run `mkdocs serve`
- Open the returned URL (typically http://127.0.0.1:8000/)