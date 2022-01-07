# syntax = docker/dockerfile:1.3
FROM node:16-alpine as builder

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
COPY package.json yarn.lock .yarnrc.yml .pnp.* ./

# Copy only necessary packages from monorepo
COPY packages/js-drive packages/js-drive
COPY packages/dapi-grpc packages/dapi-grpc
COPY packages/feature-flags-contract packages/feature-flags-contract
COPY packages/js-dpp packages/js-dpp
COPY packages/js-grpc-common packages/js-grpc-common
COPY packages/masternode-reward-shares-contract packages/masternode-reward-shares-contract

# Print build output
RUN yarn config set enableInlineBuilds true

# Install Drive-specific dependencies using previous
# node_modules directory to reuse built binaries
RUN --mount=type=cache,target=/tmp/unplugged \
    cp -R /tmp/unplugged /platform/.yarn/ && \
    yarn workspaces focus --production @dashevo/drive && \
    cp -R /platform/.yarn/unplugged /tmp/

FROM node:16-alpine

ARG NODE_ENV=production
ENV NODE_ENV ${NODE_ENV}

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Drive Node.JS"

# Install ZMQ shared library
RUN apk update && apk add --no-cache zeromq-dev

# Install latest yarn
RUN yarn set version 3.1.0

WORKDIR /platform

COPY --from=builder /platform /platform

RUN cp /platform/packages/js-drive/.env.example /platform/packages/js-drive/.env

EXPOSE 26658
