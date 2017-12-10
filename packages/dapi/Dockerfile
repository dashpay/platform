FROM 103738324493.dkr.ecr.us-west-2.amazonaws.com/dashevo/v13-node-base:latest
LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Dockerised DAPI"

RUN apk update && apk --no-cache upgrade && apk add --no-cache git openssh-client python alpine-sdk libzmq zeromq-dev

WORKDIR /dapi

# copy package manifest separately from code to avoid installing packages every
# time code is changed
COPY package.json package-lock.json /dapi/
RUN npm install

COPY . /dapi

EXPOSE 3000

CMD ["node", "/dapi/nodeStarter.js"]
