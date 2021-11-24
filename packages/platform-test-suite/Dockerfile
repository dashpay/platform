FROM node:16-alpine

ARG NODE_ENV=production
ENV NODE_ENV ${NODE_ENV}

RUN apk update && \
    apk --no-cache upgrade && \
    apk add --no-cache git \
                       openssh-client \
                       python3 \
                       alpine-sdk

# Enable corepack https://github.com/nodejs/corepack
RUN corepack enable

WORKDIR /platform

# Copy yarn files
COPY .yarn ./.yarn
COPY package.json yarn.lock .yarnrc.yml ./

# Copy only necessary packages from monorepo
COPY packages/dapi-grpc packages/dapi-grpc
COPY packages/dpns-contract packages/dpns-contract
COPY packages/dashpay-contract packages/dashpay-contract
COPY packages/feature-flags-contract packages/feature-flags-contract
COPY packages/js-dapi-client packages/js-dapi-client
COPY packages/js-dash-sdk packages/js-dash-sdk
COPY packages/js-dpp packages/js-dpp
COPY packages/wallet-lib packages/wallet-lib
COPY packages/js-grpc-common packages/js-grpc-common
COPY packages/feature-flags-contract packages/feature-flags-contract

# Install Test Suite specific dependencies using previous
# node_modules directory to reuse built binaries
RUN --mount=type=cache,target=/tmp/node_modules \
    cp -R /tmp/node_modules /platform/ && \
    yarn workspaces focus --production @dashevo/platform-test-suite && \
    cp -R node_modules /tmp/

FROM node:16-alpine

ARG NODE_ENV=production
ENV NODE_ENV ${NODE_ENV}

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="DAPI Node.JS"

# Install required deps
RUN apk add --no-cache bash \
                       git

ENV PATH /platform/node_modules/.bin:$PATH

# Set DAPI package as a current workdir
WORKDIR /platform/packages/platform-test-suite

# Copy linkedd packages and npm modules
COPY --from=node_modules /platform/packages /platform/packages
COPY --from=node_modules /platform/node_modules /platform/node_modules

RUN cp .env.example .env

EXPOSE 2500 2501 2510
