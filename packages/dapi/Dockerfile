# syntax = docker/dockerfile:1.2
FROM node:16-alpine as node_modules

RUN apk update && \
    apk --no-cache upgrade && \
    apk add --no-cache git \
                       openssh-client \
                       python3 \
                       alpine-sdk \
                       zeromq-dev

# Enable node-gyp cache
RUN npm install -g node-gyp-cache@0.2.1 && \
    npm config set node_gyp node-gyp-cache

# Update npm to version 7 to work with workspaces
RUN npm install -g npm@7

# Install npm modules
ENV npm_config_zmq_external=true

WORKDIR /platform

COPY package.json package-lock.json /platform/

COPY packages/ /platform/packages/

# Remove packages that dapi won't need
RUN rm -rf /platform/packages/drive /platform/packages/dashmate /platform/packages/platform-test-suite platform/packages/js-dash-sdk /platform/packages/wallet-lib

# Install dapi-specific dependencies
RUN --mount=type=cache,target=/root/.npm --mount=type=cache,target=/root/.cache npm ci -w @dashevo/dapi --production

FROM node:16-alpine

ARG NODE_ENV=production
ENV NODE_ENV ${NODE_ENV}

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="DAPI Node.JS"

# Install ZMQ shared library
RUN apk update && apk add --no-cache zeromq-dev

# Copy packages dir to create symlinks to modules available locally
COPY --from=node_modules /platform/packages/ /packages
# Copy hoisted NPM modules
COPY --from=node_modules /platform/node_modules/ /node_modules
COPY --from=node_modules /platform/package.json /package.json
COPY --from=node_modules /platform/package-lock.json /package-lock.json

ENV PATH /node_modules/.bin:$PATH

# Copy project files
WORKDIR /usr/src/app

COPY packages/dapi/ .
# Copy dapi-specific NPM modules that are in conflict with the root node_modules (if any)
COPY --from=node_modules /platform/packages/dapi/node_modules/ ./node_modules

RUN cp .env.example .env

EXPOSE 2500 2501 2510
