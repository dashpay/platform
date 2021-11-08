# syntax = docker/dockerfile:1.3
FROM node:16-alpine as node_gyp_cache

RUN apk update && \
    apk --no-cache upgrade && \
    apk add --no-cache git \
                      openssh-client \
                      python3 \
                      alpine-sdk \
                      zeromq-dev

WORKDIR /platform

ENV npm_config_zmq_external=true

# Install slow native modules in order to cache them for forthcomming stages
RUN --mount=type=cache,target=/root/.npm \
    npm install --no-audit --loglevel info --production --global-style true \
    zeromq@5.2.0

FROM node:16-alpine as node_modules

RUN apk update && \
    apk --no-cache upgrade && \
    apk add --no-cache git \
                       openssh-client \
                       jq

WORKDIR /platform

# Downgrade NPM to 7.18.1 to fix https://github.com/npm/cli/issues/3847https://github.com/npm/cli/issues/3847
RUN npm install --global  --production --no-audit npm@7.18.1

COPY package.json package-lock.json /platform/

# Copy only necessary packages from monorepo
COPY packages/dapi packages/dapi
COPY packages/dapi-grpc packages/dapi-grpc
COPY packages/js-dpp packages/js-dpp
COPY packages/js-grpc-common packages/js-grpc-common
COPY packages/feature-flags-contract packages/feature-flags-contract

# Do not install already built native modules
RUN echo "$( jq -M 'del(.dependencies | .zeromq)' packages/dapi/package.json )" > packages/dapi/package.json

# Install dapi-specific dependencies
RUN --mount=type=cache,target=/root/.npm \
    npm ci --no-audit --loglevel info -w @dashevo/dapi --production

FROM node:16-alpine

ARG NODE_ENV=production
ENV NODE_ENV ${NODE_ENV}

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="DAPI Node.JS"

ENV PATH /node_modules/.bin:$PATH

# Copy project files
WORKDIR /usr/src/app

# Install ZMQ shared library
RUN apk update && apk add --no-cache zeromq-dev

# Copy cached packages with native modules
COPY --from=node_gyp_cache /platform/node_modules/zeromq ./node_modules/zeromq

# Copy packages dir to create symlinks to modules available locally
COPY --from=node_modules /platform/packages/ /packages

# Copy hoisted NPM modules
COPY --from=node_modules /platform/node_modules/ /node_modules
COPY --from=node_modules /platform/package.json /package.json
COPY --from=node_modules /platform/package-lock.json /package-lock.json

# Copy app code
COPY --from=node_modules /platform/packages/dapi .

RUN cp .env.example .env

EXPOSE 2500 2501 2510
