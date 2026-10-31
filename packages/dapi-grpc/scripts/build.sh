#!/usr/bin/env bash
# Generate all published clients with the locked native toolchain.
set -euo pipefail

if [[ "${SKIP_GRPC_PROTO_BUILD:-0}" == 1 ]]; then
  echo 'WARN: Skipping GRPC protobuf definitions rebuild'
  exit 0
fi

PACKAGE_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
TOOLCHAIN=$(python3 "$PACKAGE_DIR/scripts/setup-codegen.py")
PROTOC="$TOOLCHAIN/bin/protoc"
TS_PLUGIN=$(command -v protoc-gen-ts)
command -v pbjs >/dev/null

# Keep the existing clients intact when any generator fails. Stage beneath the
# package so replacement stays on the same filesystem and Yarn PnP still works.
STAGING=$(mktemp -d "$PACKAGE_DIR/.clients-build.XXXXXX")
cleanup() {
  # Restore the previous tree if interrupted between the two renames.
  if [[ ! -e "$PACKAGE_DIR/clients" && -d "$STAGING/previous" ]]; then
    mv "$STAGING/previous" "$PACKAGE_DIR/clients"
  fi
  rm -rf "$STAGING"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
cp -R "$PACKAGE_DIR/clients" "$STAGING/clients"

for service in core drive platform; do
  output="$STAGING/clients/$service/v0"
  schema="$PACKAGE_DIR/protos/$service/v0/$service.proto"
  includes=(-I"$PACKAGE_DIR/protos/$service/v0" -I"$PACKAGE_DIR/protos" -I"$TOOLCHAIN/include")
  # Preserve handwritten PromiseClient modules and documentation.
  rm -f "$output/web/${service}_pb"* "$output/nodejs/${service}_protoc.js" "$output/nodejs/${service}_pbjs.js"
  "$PROTOC" "${includes[@]}" --plugin="protoc-gen-ts=$TS_PLUGIN" \
    --js_out="import_style=commonjs:$output/web" \
    --ts_out="service=grpc-web:$output/web" "$schema"
  cp "$output/web/${service}_pb.js" "$output/nodejs/${service}_protoc.js"
  root=platform_root
  if [[ "$service" == core ]]; then root=core_root; fi
  pbjs -t static-module -w commonjs -r "$root" -p "$PACKAGE_DIR/protos" \
    -o "$output/nodejs/${service}_pbjs.js" "$schema"
  if [[ "$service" != drive ]]; then
    for language in java objective-c python; do
      rm -rf "${output:?}/$language"
      mkdir -p "$output/$language"
    done
    "$PROTOC" "${includes[@]}" --plugin="protoc-gen-grpc-java=$TOOLCHAIN/bin/protoc-gen-grpc-java" \
      --grpc-java_out="$output/java" "$schema"
    "$PROTOC" "${includes[@]}" --plugin="protoc-gen-grpc=$TOOLCHAIN/bin/grpc_objective_c_plugin" \
      --objc_out="$output/objective-c" --grpc_out="$output/objective-c" "$schema"
    "$PROTOC" "${includes[@]}" --plugin="protoc-gen-grpc=$TOOLCHAIN/bin/grpc_python_plugin" \
      --python_out="$output/python" --grpc_out="$output/python" "$schema"
  fi
done

(cd "$STAGING" && "$PACKAGE_DIR/scripts/patch-protobuf-js.sh")
# Generation and patching have all succeeded. Preserve handwritten files by
# replacing the staged copy of the complete client tree.
mv "$PACKAGE_DIR/clients" "$STAGING/previous"
if ! mv "$STAGING/clients" "$PACKAGE_DIR/clients"; then
  mv "$STAGING/previous" "$PACKAGE_DIR/clients"
  exit 1
fi
