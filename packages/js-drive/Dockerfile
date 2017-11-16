FROM node:9-alpine
LABEL maintainer="Dash Evolution Developers <evodevs@dash.org>"
LABEL description="Dockerised DashDrive API"

RUN apk update && apk upgrade --no-cache && apk add --no-cache git openssh-client

WORKDIR /app

# copy ssh deploy key for private git repo
COPY evo-app-deploy.ed25519  /root/.ssh/id_ed25519
RUN chmod 0700 /root/.ssh/ && chmod 0600 /root/.ssh/id_ed25519
RUN echo -e "Host github.com\nStrictHostKeyChecking no" > /root/.ssh/config

# copy package manifest separately from code to avoid installing packages every
# time code is changed
COPY package.json package-lock.json /app/
RUN /usr/local/bin/npm install

COPY . /app

EXPOSE 5001

CMD ["/usr/local/bin/node", "/app/api.js"]
# CMD ["/bin/ash"]
