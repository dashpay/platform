# syntax = docker/dockerfile:1.3
FROM node:16-alpine as builder

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

WORKDIR /platform

# Copy yarn files
COPY .yarn ./.yarn
COPY package.json yarn.lock .yarnrc.yml .pnp.* ./

# Copy only necessary packages from monorepo
COPY packages/dapi packages/dapi
COPY packages/dapi-grpc packages/dapi-grpc
COPY packages/js-dpp packages/js-dpp
COPY packages/js-grpc-common packages/js-grpc-common
COPY packages/feature-flags-contract packages/feature-flags-contract
COPY packages/masternode-reward-shares-contract packages/masternode-reward-shares-contract

# Print build output
RUN yarn config set enableInlineBuilds true

# Install DAPI-specific dependencies using previous
# node_modules directory to reuse built binaries
RUN --mount=type=cache,target=/tmp/unplugged \
    cp -R /tmp/unplugged /platform/.yarn/ && \
    yarn workspaces focus --production @dashevo/dapi && \
    cp -R /platform/.yarn/unplugged /tmp/


FROM node:16-alpine

ARG NODE_ENV=production
ENV NODE_ENV ${NODE_ENV}

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="DAPI Node.JS"

# Install ZMQ shared library
RUN apk update && apk add --no-cache zeromq-dev

# Install latest yarn
RUN yarn set version 3.1.0

WORKDIR /platform

COPY --from=builder /platform /platform

RUN cp /platform/packages/dapi/.env.example /platform/packages/dapi/.env

EXPOSE 2500 2501 2510
