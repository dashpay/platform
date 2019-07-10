const path = require('path');
const grpc = require('grpc');
const protoLoader = require('@grpc/proto-loader');
const snakeCase = require('lodash.snakecase');

function loadPackageDefinition(serviceName) {
  const protoName = snakeCase(serviceName);

  const protoPath = path.join(__dirname, `../protos/${protoName}.proto`);

  const definition = protoLoader.loadSync(protoPath, {
    keepCase: false,
    longs: String,
    enums: String,
    bytes: Uint8Array,
    defaults: true,
  });

  const packageDefinition = grpc.loadPackageDefinition(definition);

  return packageDefinition.org.dash.platform.dapi;
}

module.exports = loadPackageDefinition;
