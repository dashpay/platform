FROM node:16-alpine as builder

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
COPY package.json yarn.lock .yarnrc.yml .pnp.* ./

# Copy only necessary packages from monorepo
COPY packages/dashmate packages/dashmate
COPY packages/dashpay-contract packages/dashpay-contract
COPY packages/dpns-contract packages/dpns-contract
COPY packages/js-dpp packages/js-dpp
COPY packages/feature-flags-contract packages/feature-flags-contract
COPY packages/masternode-reward-shares-contract packages/masternode-reward-shares-contract
COPY packages/wallet-lib packages/wallet-lib
COPY packages/js-dash-sdk packages/js-dash-sdk
COPY packages/js-dapi-client packages/js-dapi-client
COPY packages/js-grpc-common packages/js-grpc-common
COPY packages/dapi-grpc packages/dapi-grpc
COPY packages/dash-spv packages/dash-spv

# Print build output
RUN yarn config set enableInlineBuilds true

# Install Test Suite specific dependencies using previous
# node_modules directory to reuse built binaries
RUN --mount=type=cache,target=/tmp/unplugged \
    cp -R /tmp/unplugged /platform/.yarn/ && \
    yarn workspaces focus --production dashmate && \
    cp -R /platform/.yarn/unplugged /tmp/

FROM node:16-alpine

ARG NODE_ENV=production
ENV NODE_ENV ${NODE_ENV}

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="DAPI Helper Node.JS"


WORKDIR /platform

COPY --from=builder /platform /platform
