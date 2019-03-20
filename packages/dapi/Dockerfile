FROM node:8-alpine
LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Dockerised DAPI"

RUN apk update && apk --no-cache upgrade && apk add --no-cache git openssh-client python alpine-sdk libzmq zeromq-dev

WORKDIR /dapi

# copy package manifest separately from code to avoid installing packages every
# time code is changed
COPY package.json package-lock.json /dapi/

RUN npm ci

COPY . /dapi

EXPOSE 3000

CMD ["node", "/dapi/lib/app.js"]
