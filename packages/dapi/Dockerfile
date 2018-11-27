FROM 103738324493.dkr.ecr.us-west-2.amazonaws.com/dashevo/v13-node-base:latest
LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Dockerised DAPI"

ARG npm_token

RUN apk update && apk --no-cache upgrade && apk add --no-cache git openssh-client python alpine-sdk libzmq zeromq-dev

WORKDIR /dapi

# copy package manifest separately from code to avoid installing packages every
# time code is changed
COPY package.json /dapi/

ENV NPM_TOKEN=$npm_token
RUN echo "//registry.npmjs.org/:_authToken=$NPM_TOKEN" >> .npmrc

RUN npm i

# Token cleanup
RUN unset NPM_TOKEN
RUN rm .npmrc

COPY . /dapi

EXPOSE 3000

CMD ["node", "/dapi/lib/app.js"]
