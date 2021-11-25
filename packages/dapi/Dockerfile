# syntax = docker/dockerfile:1.3
FROM node:16-alpine as node_modules

ARG NODE_ENV=production
ENV NODE_ENV ${NODE_ENV}

RUN apk update && \
    apk --no-cache upgrade && \
    apk add --no-cache git \
                       openssh-client \
                       python3 \
                       alpine-sdk \
                       zeromq-dev

# Enable corepack https://github.com/nodejs/corepack
RUN corepack enable

# Show yarn native module build logs
RUN yarn config set enableInlineBuilds true

WORKDIR /platform

# Copy yarn files
COPY .yarn ./.yarn
COPY package.json yarn.lock .yarnrc.yml ./

# Copy only necessary packages from monorepo
COPY packages/dapi packages/dapi
COPY packages/dapi-grpc packages/dapi-grpc
COPY packages/js-dpp packages/js-dpp
COPY packages/js-grpc-common packages/js-grpc-common
COPY packages/feature-flags-contract packages/feature-flags-contract

# Install DAPI-specific dependencies using previous
# node_modules directory to reuse built binaries
RUN --mount=type=cache,target=/tmp/node_modules \
    cp -R /tmp/node_modules /platform/ && \
    yarn workspaces focus --production @dashevo/dapi && \
    cp -R node_modules /tmp/


FROM node:16-alpine

ARG NODE_ENV=production
ENV NODE_ENV ${NODE_ENV}

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="DAPI Node.JS"

# Install ZMQ shared library
RUN apk update && apk add --no-cache zeromq-dev

ENV PATH /platform/node_modules/.bin:$PATH

# Set DAPI package as a current workdir
WORKDIR /platform/packages/dapi

# Copy linkedd packages and npm modules
COPY --from=node_modules /platform/packages /platform/packages
COPY --from=node_modules /platform/node_modules /platform/node_modules

RUN cp .env.example .env

EXPOSE 2500 2501 2510
