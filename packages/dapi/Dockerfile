FROM node:8-wheezy
LABEL maintainer="Dash Evolution Developers <evodevs@dash.org>"
LABEL description="Dockerised DAPI"

RUN apt-get update && apt-get -y upgrade

WORKDIR /dapi

# copy package manifest separately from code to avoid installing packages every
# time code is changed
COPY package.json package-lock.json /dapi/

COPY . /dapi

EXPOSE 3000

CMD ["node", "/dapi/nodeStarter.js"]