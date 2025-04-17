const path = require('path');

const { loadPackageDefinition } = require('@dashevo/grpc-common');

function getDriveDefinition(version) {
  const protoPath = path.join(__dirname, `../protos/drive/v${version}/drive.proto`);
  const includeDirs = [
    path.join(__dirname, '../protos/'),
  ];

  return loadPackageDefinition(
    protoPath,
    `org.dash.platform.drive.v${version}.DriveInternal`,
    includeDirs,
  );
}

module.exports = getDriveDefinition;
