FROM node:8-stretch

RUN apt-get -qq update && apt-get -qq install -y \
  unzip

RUN git clone https://github.com/grpc/grpc-web /github/grpc-web

RUN curl -sSL https://github.com/protocolbuffers/protobuf/releases/download/v3.6.1/\
protoc-3.6.1-linux-x86_64.zip -o /tmp/protoc.zip && \
  cd /tmp && \
  unzip -qq protoc.zip && \
  cp /tmp/bin/protoc /usr/local/bin/protoc

RUN curl -sSL https://github.com/grpc/grpc-web/releases/download/1.0.4/\
protoc-gen-grpc-web-1.0.4-linux-x86_64 -o /usr/local/bin/protoc-gen-grpc-web && \
  chmod +x /usr/local/bin/protoc-gen-grpc-web

RUN npm i -g protobufjs
