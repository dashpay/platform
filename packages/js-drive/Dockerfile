FROM 103738324493.dkr.ecr.us-west-2.amazonaws.com/dashevo/v13-node-base:latest
LABEL maintainer="Dash Developers <dev@dash.org>"
LABEL description="Dockerised DashDrive API"

RUN apk update && apk upgrade --no-cache && apk add --no-cache git openssh-client

WORKDIR /app

# copy package manifest separately from code to avoid installing packages every
# time code is changed
COPY package.json package-lock.json /app/
RUN /usr/local/bin/npm install

COPY . /app

EXPOSE 5001

CMD ["/usr/local/bin/node", "/app/lib/api.js"]
# CMD ["/bin/ash"]
