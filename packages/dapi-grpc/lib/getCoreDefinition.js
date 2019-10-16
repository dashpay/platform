const path = require('path');

const { loadPackageDefinition } = require('@dashevo/grpc-common');

function getCoreDefinition() {
  const protoPath = path.join(__dirname, '../protos/core.proto');

  return loadPackageDefinition(protoPath, 'org.dash.platform.dapi.v0.Core');
}

module.exports = getCoreDefinition;
