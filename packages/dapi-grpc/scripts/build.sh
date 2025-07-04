#!/usr/bin/env bash
# shellcheck disable=SC2250
#

SKIP_GRPC_PROTO_BUILD=${SKIP_GRPC_PROTO_BUILD:-0}
if [[ "${SKIP_GRPC_PROTO_BUILD}" == "1" ]]; then
  echo WARN: Skipping GRPC protobuf definitions rebuild
  exit 0
fi

PROTOS_PATH="$PWD/protos"

CORE_PROTO_PATH="$PWD/protos/core/v0"
CORE_CLIENTS_PATH="$PWD/clients/core/v0"

PLATFORM_PROTO_PATH="$PWD/protos/platform/v0"
PLATFORM_CLIENTS_PATH="$PWD/clients/platform/v0"

DRIVE_PROTO_PATH="$PWD/protos/drive/v0"
DRIVE_CLIENTS_PATH="$PWD/clients/drive/v0"

CORE_WEB_OUT_PATH="$CORE_CLIENTS_PATH/web"
PLATFORM_WEB_OUT_PATH="$PLATFORM_CLIENTS_PATH/web"
DRIVE_WEB_OUT_PATH="$DRIVE_CLIENTS_PATH/web"

CORE_JAVA_OUT_PATH="$CORE_CLIENTS_PATH/java"
PLATFORM_JAVA_OUT_PATH="$PLATFORM_CLIENTS_PATH/java"

CORE_OBJ_C_OUT_PATH="$CORE_CLIENTS_PATH/objective-c"
PLATFORM_OBJ_C_OUT_PATH="$PLATFORM_CLIENTS_PATH/objective-c"

CORE_PYTHON_OUT_PATH="$CORE_CLIENTS_PATH/python"
PLATFORM_PYTHON_OUT_PATH="$PLATFORM_CLIENTS_PATH/python"

PROTOC_IMAGE="rvolosatovs/protoc:4.0.0"

set -ex

#################################################
# Generate JavaScript client for `Core` service #
#################################################

rm -rf "${CORE_WEB_OUT_PATH:?}/*" || true

docker run -v "$CORE_PROTO_PATH:$CORE_PROTO_PATH" \
  -v "$CORE_WEB_OUT_PATH:$CORE_WEB_OUT_PATH" \
  --rm \
  "$PROTOC_IMAGE" \
  --js_out="import_style=commonjs:$CORE_WEB_OUT_PATH" \
  --ts_out="service=grpc-web:$CORE_WEB_OUT_PATH" \
  -I="$CORE_PROTO_PATH" \
  "core.proto"

# Clean node message classes

rm -rf "$CORE_CLIENTS_PATH/nodejs/*_protoc.js" || true
rm -rf "$CORE_CLIENTS_PATH/nodejs/*_pbjs.js" || true

# Copy compiled modules with message classes

cp "$CORE_WEB_OUT_PATH/core_pb.js" "$CORE_CLIENTS_PATH/nodejs/core_protoc.js"

# Generate node message classes
pbjs \
  -t static-module \
  -w commonjs \
  -r core_root \
  -o "$CORE_CLIENTS_PATH/nodejs/core_pbjs.js" \
  "$CORE_PROTO_PATH/core.proto"

#####################################################
# Generate JavaScript client for `DriveInternal` service #
#####################################################

rm -rf "${DRIVE_WEB_OUT_PATH:?}/*" || true

docker run -v "$DRIVE_PROTO_PATH:$DRIVE_PROTO_PATH" \
  -v "$DRIVE_WEB_OUT_PATH:$DRIVE_WEB_OUT_PATH" \
  -v "$PROTOS_PATH:$PROTOS_PATH" \
  --rm \
  "$PROTOC_IMAGE" \
  --js_out="import_style=commonjs:$DRIVE_WEB_OUT_PATH" \
  --ts_out="service=grpc-web:$DRIVE_WEB_OUT_PATH" \
  -I="$DRIVE_PROTO_PATH" \
  -I="$PROTOS_PATH" \
  "drive.proto"

# Clean node message classes

rm -rf "$DRIVE_CLIENTS_PATH/nodejs/*_protoc.js" || true
rm -rf "$DRIVE_CLIENTS_PATH/nodejs/*_pbjs.js" || true

# Copy compiled modules with message classes

cp "$DRIVE_WEB_OUT_PATH/drive_pb.js" "$DRIVE_CLIENTS_PATH/nodejs/drive_protoc.js"

pbjs \
  -t static-module \
  -w commonjs \
  -r platform_root \
  -p "$PROTOS_PATH" \
  -o "$DRIVE_CLIENTS_PATH/nodejs/drive_pbjs.js" \
  "$DRIVE_PROTO_PATH/drive.proto"

#####################################################
# Generate JavaScript client for `Platform` service #
#####################################################

rm -rf "${PLATFORM_WEB_OUT_PATH:?}/*" || true

docker run -v "$PLATFORM_PROTO_PATH:$PLATFORM_PROTO_PATH" \
  -v "$PLATFORM_WEB_OUT_PATH:$PLATFORM_WEB_OUT_PATH" \
  -v "$PROTOS_PATH:$PROTOS_PATH" \
  --rm \
  "$PROTOC_IMAGE" \
  --js_out="import_style=commonjs:$PLATFORM_WEB_OUT_PATH" \
  --ts_out="service=grpc-web:$PLATFORM_WEB_OUT_PATH" \
  -I="$PLATFORM_PROTO_PATH" \
  -I="$PROTOS_PATH" \
  "platform.proto"

# Clean node message classes

rm -rf "$PLATFORM_CLIENTS_PATH/nodejs/*_protoc.js" || true
rm -rf "$PLATFORM_CLIENTS_PATH/nodejs/*_pbjs.js" || true

# Copy compiled modules with message classes

cp "$PLATFORM_WEB_OUT_PATH/platform_pb.js" "$PLATFORM_CLIENTS_PATH/nodejs/platform_protoc.js"

pbjs \
  -t static-module \
  -w commonjs \
  -r platform_root \
  -o "$PLATFORM_CLIENTS_PATH/nodejs/platform_pbjs.js" \
  -p "$PROTOS_PATH" \
  "$PLATFORM_PROTO_PATH/platform.proto"

###################################
# Generate Java client for `Core` #
###################################

rm -rf "${CORE_JAVA_OUT_PATH:?}/*" || true

docker run -v "$CORE_PROTO_PATH:$CORE_PROTO_PATH" \
  -v "$CORE_JAVA_OUT_PATH:$CORE_JAVA_OUT_PATH" \
  -v "$PROTOS_PATH:$PROTOS_PATH" \
  --rm \
  "$PROTOC_IMAGE" \
  --plugin=protoc-gen-grpc=/usr/bin/protoc-gen-grpc-java \
  --grpc-java_out="$CORE_JAVA_OUT_PATH" \
  --proto_path="$CORE_PROTO_PATH" \
  -I="$CORE_PROTO_PATH" \
  -I="$PROTOS_PATH" \
  "core.proto"

#######################################
# Generate Java client for `Platform` #
#######################################

rm -rf "${PLATFORM_JAVA_OUT_PATH:?}/*" || true

docker run -v "$PLATFORM_PROTO_PATH:$PLATFORM_PROTO_PATH" \
  -v "$PLATFORM_JAVA_OUT_PATH:$PLATFORM_JAVA_OUT_PATH" \
  -v "$PROTOS_PATH:$PROTOS_PATH" \
  --rm \
  "$PROTOC_IMAGE" \
  --plugin=protoc-gen-grpc=/usr/bin/protoc-gen-grpc-java \
  --grpc-java_out="$PLATFORM_JAVA_OUT_PATH" \
  --proto_path="$PLATFORM_PROTO_PATH" \
  -I="$PLATFORM_PROTO_PATH" \
  -I="$PROTOS_PATH" \
  "platform.proto"

##########################################
# Generate Objective-C client for `Core` #
##########################################

rm -rf "${CORE_OBJ_C_OUT_PATH:?}/*" || true

docker run -v "$CORE_PROTO_PATH:$CORE_PROTO_PATH" \
  -v "$CORE_OBJ_C_OUT_PATH:$CORE_OBJ_C_OUT_PATH" \
  -v "$PROTOS_PATH:$PROTOS_PATH" \
  --rm \
  "$PROTOC_IMAGE" \
  --plugin=protoc-gen-grpc=/usr/bin/grpc_objective_c_plugin \
  --objc_out="$CORE_OBJ_C_OUT_PATH" \
  --grpc_out="$CORE_OBJ_C_OUT_PATH" \
  --proto_path="$CORE_PROTO_PATH" \
  -I="$CORE_PROTO_PATH" \
  -I="$PROTOS_PATH" \
  "core.proto"

##############################################
# Generate Objective-C client for `Platform` #
##############################################

rm -rf "${PLATFORM_OBJ_C_OUT_PATH:?}/*" || true

docker run -v "$PLATFORM_PROTO_PATH:$PLATFORM_PROTO_PATH" \
  -v "$PLATFORM_OBJ_C_OUT_PATH:$PLATFORM_OBJ_C_OUT_PATH" \
  -v "$PROTOS_PATH:$PROTOS_PATH" \
  --rm \
  "$PROTOC_IMAGE" \
  --plugin=protoc-gen-grpc=/usr/bin/grpc_objective_c_plugin \
  --objc_out="$PLATFORM_OBJ_C_OUT_PATH" \
  --grpc_out="$PLATFORM_OBJ_C_OUT_PATH" \
  --proto_path="$PLATFORM_PROTO_PATH" \
  -I="$PLATFORM_PROTO_PATH" \
  -I="$PROTOS_PATH" \
  "platform.proto"

#####################################
# Generate Python client for `Core` #
#####################################

rm -rf "${CORE_PYTHON_OUT_PATH:?}/*" || true

docker run -v "$CORE_PROTO_PATH:$CORE_PROTO_PATH" \
  -v "$CORE_PYTHON_OUT_PATH:$CORE_PYTHON_OUT_PATH" \
  -v "$PROTOS_PATH:$PROTOS_PATH" \
  --rm \
  "$PROTOC_IMAGE" \
  --plugin=protoc-gen-grpc=/usr/bin/grpc_python_plugin \
  --python_out="$CORE_PYTHON_OUT_PATH" \
  --grpc_out="$CORE_PYTHON_OUT_PATH" \
  --proto_path="$CORE_PROTO_PATH" \
  -I="$CORE_PROTO_PATH" \
  -I="$PROTOS_PATH" \
  "core.proto"

#########################################
# Generate Python client for `Platform` #
#########################################

rm -rf "${PLATFORM_PYTHON_OUT_PATH:?}/*" || true

docker run -v "$PLATFORM_PROTO_PATH:$PLATFORM_PROTO_PATH" \
  -v "$PLATFORM_PYTHON_OUT_PATH:$PLATFORM_PYTHON_OUT_PATH" \
  -v "$PROTOS_PATH:$PROTOS_PATH" \
  --rm \
  "$PROTOC_IMAGE" \
  --plugin=protoc-gen-grpc=/usr/bin/grpc_python_plugin \
  --python_out="$PLATFORM_PYTHON_OUT_PATH" \
  --grpc_out="$PLATFORM_PYTHON_OUT_PATH" \
  --proto_path="$PLATFORM_PROTO_PATH" \
  -I="$PLATFORM_PROTO_PATH" \
  -I="$PROTOS_PATH" \
  "platform.proto"

# Patch generated protobuf files
exec "${PWD}/scripts/patch-protobuf-js.sh"
