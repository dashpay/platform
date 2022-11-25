const { grpc: { Metadata } } = require('@improbable-eng/grpc-web');
const CorePromiseClient = require('./clients/core/v0/web/CorePromiseClient');
const PlatformPromiseClient = require('./clients/platform/v0/web/PlatformPromiseClient');

const coreMessages = require('./clients/core/v0/web/core_pb');
const platformMessages = require('./clients/platform/v0/web/platform_pb');
const parseMetadata = require('./lib/utils/parseMetadata');

module.exports = {
  v0: {
    ...coreMessages,
    ...platformMessages,
    CorePromiseClient,
    PlatformPromiseClient,
  },
  parseMetadata,
  Metadata,
};
