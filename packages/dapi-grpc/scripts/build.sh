#!/usr/bin/env bash

# Build "grpcweb/common" until it is updated regularly on Docker Hub

docker build -t grpcweb/common .

# Generate GRPC-Web client for `TransactionsFilterStream` service

OUT_PATH="$PWD/dist/web"

mkdir -p dist/web

docker run -v "$PWD:$PWD" \
           grpcweb/common \
           protoc -I="$PWD" "tx_filter_stream.proto" \
                   --js_out="import_style=commonjs:$OUT_PATH" \
                   --grpc-web_out="import_style=commonjs,mode=grpcwebtext:$OUT_PATH"
