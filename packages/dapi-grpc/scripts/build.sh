#!/usr/bin/env bash

# Generate GRPC-Web client for `Core` service

PROTO_PATH="$PWD/protos"
CLIENTS_PATH="$PWD/clients"

WEB_OUT_PATH="$CLIENTS_PATH/web"

rm -rf "$WEB_OUT_PATH/*"

docker run -v "$PROTO_PATH:$PROTO_PATH" \
           -v "$WEB_OUT_PATH:$WEB_OUT_PATH" \
           --rm \
           grpcweb/common \
           protoc -I="$PROTO_PATH" "core.proto" \
                   --js_out="import_style=commonjs:$WEB_OUT_PATH" \
                   --grpc-web_out="import_style=commonjs,mode=grpcwebtext:$WEB_OUT_PATH"

# Generate GRPC-Web client for `Platform` service

docker run -v "$PROTO_PATH:$PROTO_PATH" \
           -v "$WEB_OUT_PATH:$WEB_OUT_PATH" \
           --rm \
           grpcweb/common \
           protoc -I="$PROTO_PATH" "platform.proto" \
                   --js_out="import_style=commonjs:$WEB_OUT_PATH" \
                   --grpc-web_out="import_style=commonjs,mode=grpcwebtext:$WEB_OUT_PATH"

# Generate GRPC-Web client for `TransactionsFilterStream` service

docker run -v "$PROTO_PATH:$PROTO_PATH" \
           -v "$WEB_OUT_PATH:$WEB_OUT_PATH" \
           --rm \
           grpcweb/common \
           protoc -I="$PROTO_PATH" "transactions_filter_stream.proto" \
                   --js_out="import_style=commonjs:$WEB_OUT_PATH" \
                   --grpc-web_out="import_style=commonjs,mode=grpcwebtext:$WEB_OUT_PATH"

# Clean node message classes

rm -rf "$CLIENTS_PATH/nodejs/*_protoc.js"
rm -rf "$CLIENTS_PATH/nodejs/*_pbjs.js"

# Copy compiled modules with message classes

cp "$WEB_OUT_PATH/core_pb.js" "$CLIENTS_PATH/nodejs/core_protoc.js"
cp "$WEB_OUT_PATH/platform_pb.js" "$CLIENTS_PATH/nodejs/platform_protoc.js"
cp "$WEB_OUT_PATH/transactions_filter_stream_pb.js" "$CLIENTS_PATH/nodejs/transactions_filter_stream_protoc.js"

# Generate node message classes
$PWD/node_modules/protobufjs/bin/pbjs \
  -t static-module \
  -w commonjs \
  -r core_root \
  -o "$CLIENTS_PATH/nodejs/core_pbjs.js" \
  "$PROTO_PATH/core.proto"

$PWD/node_modules/protobufjs/bin/pbjs \
  -t static-module \
  -w commonjs \
  -r platform_root \
  -o "$CLIENTS_PATH/nodejs/platform_pbjs.js" \
  "$PROTO_PATH/platform.proto"

$PWD/node_modules/protobufjs/bin/pbjs \
  -t static-module \
  -w commonjs \
  -r transactions_filter_stream_root \
  -o "$CLIENTS_PATH/nodejs/transactions_filter_stream_pbjs.js" \
  "$PROTO_PATH/transactions_filter_stream.proto"

# Generate GRPC Java client for `Core`

JAVA_OUT_PATH="$CLIENTS_PATH/java"

rm -rf "$JAVA_OUT_PATH/*"

docker run -v "$PROTO_PATH:$PROTO_PATH" \
           -v "$JAVA_OUT_PATH:$JAVA_OUT_PATH" \
           --rm \
           znly/protoc \
           --plugin=protoc-gen-grpc=/usr/bin/protoc-gen-grpc-java \
           --grpc-java_out="$JAVA_OUT_PATH" \
           --proto_path="$PROTO_PATH" \
           -I "$PROTO_PATH" \
           "core.proto"

# Generate GRPC Java client for `Platform`

docker run -v "$PROTO_PATH:$PROTO_PATH" \
           -v "$JAVA_OUT_PATH:$JAVA_OUT_PATH" \
           --rm \
           znly/protoc \
           --plugin=protoc-gen-grpc=/usr/bin/protoc-gen-grpc-java \
           --grpc-java_out="$JAVA_OUT_PATH" \
           --proto_path="$PROTO_PATH" \
           -I "$PROTO_PATH" \
           "platform.proto"

# Generate GRPC Java client for `TransactionsFilterStream`

docker run -v "$PROTO_PATH:$PROTO_PATH" \
           -v "$JAVA_OUT_PATH:$JAVA_OUT_PATH" \
           --rm \
           znly/protoc \
           --plugin=protoc-gen-grpc=/usr/bin/protoc-gen-grpc-java \
           --grpc-java_out="$JAVA_OUT_PATH" \
           --proto_path="$PROTO_PATH" \
           -I "$PROTO_PATH" \
           "transactions_filter_stream.proto"

# Generate GRPC Objective-C client for `Core`

OBJ_C_OUT_PATH="$CLIENTS_PATH/objective-c"

rm -rf "$OBJ_C_OUT_PATH/*"

docker run -v "$PROTO_PATH:$PROTO_PATH" \
           -v "$OBJ_C_OUT_PATH:$OBJ_C_OUT_PATH" \
           --rm \
           znly/protoc \
           --plugin=protoc-gen-grpc=/usr/bin/grpc_objective_c_plugin \
           --objc_out="$OBJ_C_OUT_PATH" \
           --grpc_out="$OBJ_C_OUT_PATH" \
           --proto_path="$PROTO_PATH" \
           -I "$PROTO_PATH" \
           "core.proto"

# Generate GRPC Objective-C client for `Platform`

docker run -v "$PROTO_PATH:$PROTO_PATH" \
           -v "$OBJ_C_OUT_PATH:$OBJ_C_OUT_PATH" \
           --rm \
           znly/protoc \
           --plugin=protoc-gen-grpc=/usr/bin/grpc_objective_c_plugin \
           --objc_out="$OBJ_C_OUT_PATH" \
           --grpc_out="$OBJ_C_OUT_PATH" \
           --proto_path="$PROTO_PATH" \
           -I "$PROTO_PATH" \
           "platform.proto"

# Generate GRPC Objective-C client for `TransactionsFilterStream`

docker run -v "$PROTO_PATH:$PROTO_PATH" \
           -v "$OBJ_C_OUT_PATH:$OBJ_C_OUT_PATH" \
           --rm \
           znly/protoc \
           --plugin=protoc-gen-grpc=/usr/bin/grpc_objective_c_plugin \
           --objc_out="$OBJ_C_OUT_PATH" \
           --grpc_out="$OBJ_C_OUT_PATH" \
           --proto_path="$PROTO_PATH" \
           -I "$PROTO_PATH" \
           "transactions_filter_stream.proto"
