FROM node:12-alpine as node_modules

RUN apk update && \
    apk --no-cache upgrade && \
    apk add --no-cache linux-headers \
                       git \
                       openssh-client \
                       python \
                       alpine-sdk \
                       zeromq-dev

# Enable node-gyp cache
RUN npm install -g node-gyp-cache@0.2.1 && \
    npm config set node_gyp node-gyp-cache

# Copy node gyp cache
COPY docker/cache/.cache /root/.cache

# Copy NPM cache
COPY docker/cache/.npm /root/.npm

# Install npm modules
ENV npm_config_zmq_external=true

COPY package.json package-lock.json /

RUN npm ci --production

FROM node:12-alpine

ARG NODE_ENV=production
ENV NODE_ENV ${NODE_ENV}

LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Drive Node.JS"

RUN apk update && apk add --no-cache zeromq-dev

# Copy NPM modules
COPY --from=node_modules /node_modules/ /node_modules
COPY --from=node_modules /package.json /package.json
COPY --from=node_modules /package-lock.json /package-lock.json

ENV PATH /node_modules/.bin:$PATH

# Copy project files
WORKDIR /usr/src/app

COPY . .

RUN cp .env.example .env

EXPOSE 26658
