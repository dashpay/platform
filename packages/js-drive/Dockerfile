FROM node:9-alpine
LABEL maintainer="Dash Evolution Developers <evodevs@dash.org>"
LABEL description="Dockerised DashDrive API"

RUN apk update && apk upgrade --no-cache

WORKDIR /app
COPY . /app
RUN /usr/local/bin/npm install

EXPOSE 5001

CMD ["/usr/local/bin/node", "/app/api.js"]
# CMD ["/bin/ash"]
