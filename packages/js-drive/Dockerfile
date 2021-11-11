# syntax = docker/dockerfile:1.3
FROM node:16-alpine as node_gyp_cache

RUN apk update && \
    apk --no-cache upgrade && \
    apk add --no-cache linux-headers \
            git \
            openssh-client \
            python3 \
            alpine-sdk \
            cmake \
            zeromq-dev

WORKDIR /platform

# Replacing github url https://github.com/actions/setup-node/issues/214
RUN git config --global url."https://github.com/".insteadOf ssh://git@github.com/

ENV npm_config_zmq_external=true

# Install native modules in order to cache them for forthcomming stages
RUN --mount=type=cache,target=/root/.npm \
    npm install --no-audit --loglevel info --production --global-style true \
    github:dashevo/node-merk \
    blake3@2.1.4 \
    level-rocksdb@4.0.0 \
    zeromq@5.2.0

FROM node:16-alpine as node_modules

RUN apk update && \
    apk --no-cache upgrade && \
    apk add --no-cache git \
                      openssh-client \
                      jq

WORKDIR /platform

# Replacing github url https://github.com/actions/setup-node/issues/214
RUN git config --global url."https://github.com/".insteadOf ssh://git@github.com/

# Downgrade NPM to 7.18.1 to fix https://github.com/npm/cli/issues/3847https://github.com/npm/cli/issues/3847
# and install build dependecies
RUN npm install --global  --production --no-audit npm@7.18.1

COPY package.json package-lock.json /platform/

# Copy only necessary packages from monorepo
COPY packages/js-drive packages/js-drive
COPY packages/dapi-grpc packages/dapi-grpc
COPY packages/feature-flags-contract packages/feature-flags-contract
COPY packages/js-dpp packages/js-dpp
COPY packages/js-grpc-common packages/js-grpc-common

# Do not install already built native modules
RUN echo "$( jq -M 'del(.dependencies | ."@dashevo/merk", .zeromq, ."level-rocksdb", .blake3)' packages/js-drive/package.json )" > packages/js-drive/package.json

# Install drive-specific dependencies
RUN --mount=type=cache,target=/root/.npm \
    npm ci --loglevel info --no-audit -w @dashevo/drive --production

FROM node:16-alpine

ARG NODE_ENV=production
ENV NODE_ENV ${NODE_ENV}

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Drive Node.JS"

ENV PATH /node_modules/.bin:$PATH

# Copy project files
WORKDIR /usr/src/app

RUN apk update && apk add --no-cache zeromq-dev

# Copy cached packages with native modules
COPY --from=node_gyp_cache /platform/node_modules/zeromq ./node_modules/zeromq
COPY --from=node_gyp_cache /platform/node_modules/@dashevo/merk ./node_modules/@dashevo/merk
COPY --from=node_gyp_cache /platform/node_modules/level-rocksdb ./node_modules/level-rocksdb
COPY --from=node_gyp_cache /platform/node_modules/blake3 ./node_modules/blake3

# Copy packages dir to create symlinks to modules available locally
COPY --from=node_modules /platform/packages/ /packages

# Copy hoisted NPM modules
COPY --from=node_modules /platform/node_modules/ /node_modules
COPY --from=node_modules /platform/package.json /package.json
COPY --from=node_modules /platform/package-lock.json /package-lock.json

# Copy app code
COPY --from=node_modules /platform/packages/js-drive .

RUN cp .env.example .env

EXPOSE 26658
