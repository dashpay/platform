const path = require('path');

const { loadPackageDefinition } = require('@dashevo/grpc-common');

function getDriveDefinition(version) {
  const protoPath = path.join(__dirname, `../protos/drive/v${version}/drive.proto`);

  return loadPackageDefinition(protoPath, `org.dash.platform.drive.v${version}.DriveInternal`);
}

module.exports = getDriveDefinition;
