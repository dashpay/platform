const CorePromiseClient = require('./clients/core/v0/nodejs/CorePromiseClient');
const PlatformPromiseClient = require('./clients/platform/v0/nodejs/PlatformPromiseClient');

const protocCoreMessages = require('./clients/core/v0/nodejs/core_protoc');
const protocPlatformMessages = require('./clients/platform/v0/nodejs/platform_protoc');

const getCoreDefinition = require('./lib/getCoreDefinition');
const getPlatformDefinition = require('./lib/getPlatformDefinition');

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

module.exports = {
  getCoreDefinition,
  getPlatformDefinition,
  v0: Object.assign({
    CorePromiseClient,
    PlatformPromiseClient,
    pbjs: Object.assign(
      {},
      pbjsCoreMessages,
      pbjsPlatformMessages,
    ),
  }, protocCoreMessages, protocPlatformMessages),
};
