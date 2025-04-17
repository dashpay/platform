const { Metadata } = require('@grpc/grpc-js/build/src/metadata');
const CorePromiseClient = require('./clients/core/v0/nodejs/CorePromiseClient');
const PlatformPromiseClient = require('./clients/platform/v0/nodejs/PlatformPromiseClient');
const DrivePromiseClient = require('./clients/drive/v0/nodejs/DrivePromiseClient');

const protocCoreMessages = require('./clients/core/v0/nodejs/core_protoc');
const protocPlatformMessages = require('./clients/platform/v0/nodejs/platform_protoc');
const protocDriveMessages = require('./clients/drive/v0/nodejs/drive_protoc');

const getCoreDefinition = require('./lib/getCoreDefinition');
const getPlatformDefinition = require('./lib/getPlatformDefinition');
const getDriveDefinition = require('./lib/getDriveDefinition');
const parseMetadata = require('./lib/utils/parseMetadata');

const {
  org: {
    dash: {
      platform: {
        dapi: {
          v0: pbjsCoreMessages,
        },
      },
    },
  },
} = require('./clients/core/v0/nodejs/core_pbjs');

const {
  org: {
    dash: {
      platform: {
        dapi: {
          v0: pbjsPlatformMessages,
        },
      },
    },
  },
} = require('./clients/platform/v0/nodejs/platform_pbjs');

const {
  org: {
    dash: {
      platform: {
        drive: {
          v0: pbjsDriveMessages,
        },
      },
    },
  },
} = require('./clients/drive/v0/nodejs/drive_pbjs');

module.exports = {
  getCoreDefinition,
  getPlatformDefinition,
  getDriveDefinition,
  v0: {
    CorePromiseClient,
    PlatformPromiseClient,
    DrivePromiseClient,
    pbjs: {

      ...pbjsCoreMessages,
      ...pbjsPlatformMessages,
      ...pbjsDriveMessages,
    },
    ...protocCoreMessages,
    ...protocPlatformMessages,
    ...protocDriveMessages,
  },
  parseMetadata,
  Metadata,
};
