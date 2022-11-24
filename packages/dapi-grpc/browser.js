const { grpc: { Metadata } } = require('@improbable-eng/grpc-web');
const CorePromiseClient = require('./clients/core/v0/web/CorePromiseClient');
const PlatformPromiseClient = require('./clients/platform/v0/web/PlatformPromiseClient');

const coreMessages = require('./clients/core/v0/web/core_pb');
const platformMessages = require('./clients/platform/v0/web/platform_pb');

const parseMetadata = (metadata) => {
  if (metadata instanceof Metadata) {
    const parsedMetadata = {};
    metadata.forEach((key, values) => {
      // Join with comma because metadata items
      // represented by an array of browser headers
      parsedMetadata[key] = values.join(', ');
    });

    return parsedMetadata;
  }

  return metadata;
};

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
