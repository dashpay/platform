FROM node:9-alpine
LABEL maintainer="Dash Evolution Developers <evodevs@dash.org>"
LABEL description="Dockerised DashDrive API"

RUN apk update && apk upgrade --no-cache

WORKDIR /app

# copy package manifest separately from code to avoid installing packages every
# time code is changed
COPY package.json package-lock.json /app/
RUN /usr/local/bin/npm install

COPY . /app

EXPOSE 5001

CMD ["/usr/local/bin/node", "/app/api.js"]
# CMD ["/bin/ash"]
