# syntax = docker/dockerfile:1.3
FROM node:16-alpine as node_modules

ARG NODE_ENV=production
ENV NODE_ENV ${NODE_ENV}

RUN apk update && \
    apk --no-cache upgrade && \
    apk add --no-cache git \
                      openssh-client \
                      linux-headers \
                      python3 \
                      alpine-sdk \
                      cmake \
                      zeromq-dev

# Enable corepack https://github.com/nodejs/corepack
RUN corepack enable

WORKDIR /platform

# Copy yarn files
COPY .yarn /platform/.yarn
COPY package.json yarn.lock .yarnrc.yml /platform/

# Copy only necessary packages from monorepo
COPY packages/js-drive packages/js-drive
COPY packages/dapi-grpc packages/dapi-grpc
COPY packages/feature-flags-contract packages/feature-flags-contract
COPY packages/js-dpp packages/js-dpp
COPY packages/js-grpc-common packages/js-grpc-common

# Install Drive-specific dependencies using previous
# node_modules directory to reuse built binaries
RUN --mount=type=cache,target=/tmp/node_modules \
    cp -R /tmp/node_modules /platform/ && \
    yarn workspaces focus --production @dashevo/drive && \
    cp -R node_modules /tmp/

FROM node:16-alpine

ARG NODE_ENV=production
ENV NODE_ENV ${NODE_ENV}

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Drive Node.JS"

RUN apk update && apk add --no-cache zeromq-dev

ENV PATH /platform/node_modules/.bin:$PATH

# Set Drive package as a current workdir
WORKDIR /platform/packages/js-drive

# Copy packages dir to create symlinks to modules available locally
COPY --from=node_modules /platform/packages /platform/packages
COPY --from=node_modules /platform/node_modules /platform/node_modules

RUN cp .env.example .env

EXPOSE 26658
