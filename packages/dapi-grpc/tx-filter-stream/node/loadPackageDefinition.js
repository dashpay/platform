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

  const packageDefinition = grpc.loadPackageDefinition(definition);

  return packageDefinition.org.dash.platform.dapi;
}

module.exports = loadPackageDefinition;
