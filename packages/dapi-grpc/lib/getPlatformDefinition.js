const path = require('path');

const { loadPackageDefinition } = require('@dashevo/grpc-common');

function getPlatformDefinition() {
  const protoPath = path.join(__dirname, '../protos/platform.proto');

  return loadPackageDefinition(protoPath, 'org.dash.platform.dapi.v0.Platform');
}

module.exports = getPlatformDefinition;
