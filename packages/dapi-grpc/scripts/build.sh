#!/usr/bin/env bash

# Build "grpcweb/common" until it is updated regularly on Docker Hub

docker build -t grpcweb/common .

# Generate GRPC-Web client for `TransactionsFilterStream` service

INCLUDE_PATH="$PWD/transactions-filter-stream"
OUT_PATH="$INCLUDE_PATH/web"

docker run -v "$INCLUDE_PATH:$INCLUDE_PATH" \
           --rm \
           grpcweb/common \
           protoc -I="$INCLUDE_PATH" "transactions_filter_stream.proto" \
                   --js_out="import_style=commonjs:$OUT_PATH" \
                   --grpc-web_out="import_style=commonjs,mode=grpcwebtext:$OUT_PATH"

cp "$OUT_PATH/transactions_filter_stream_pb.js" "$INCLUDE_PATH/node/"

# Generate GRPC-Web client for `Core` service

INCLUDE_PATH="$PWD/core"
OUT_PATH="$INCLUDE_PATH/web"

docker run -v "$INCLUDE_PATH:$INCLUDE_PATH" \
           --rm \
           grpcweb/common \
           protoc -I="$INCLUDE_PATH" "core.proto" \
                   --js_out="import_style=commonjs:$OUT_PATH" \
                   --grpc-web_out="import_style=commonjs,mode=grpcwebtext:$OUT_PATH"

cp "$OUT_PATH/core_pb.js" "$INCLUDE_PATH/node/"
