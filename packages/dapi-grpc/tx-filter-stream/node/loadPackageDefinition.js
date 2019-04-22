const path = require('path');
const grpc = require('grpc');
const protoLoader = require('@grpc/proto-loader');

const protoPath = path.join(__dirname, '../tx_filter_stream.proto');

function loadPackageDefinition() {
  const definition = protoLoader.loadSync(protoPath, {
    keepCase: true,
    longs: String,
    enums: String,
    bytes: Array,
    defaults: true,
  });

  return grpc.loadPackageDefinition(definition);
}

module.exports = loadPackageDefinition();
