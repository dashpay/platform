const { CoreClient: CorePromiseClient } = require('./clients/core/v0/web/core_pb_service');
const { PlatformClient: PlatformPromiseClient } = require('./clients/platform/v0/web/platform_pb_service');

const coreMessages = require('./clients/core/v0/web/core_pb');
const platformMessages = require('./clients/platform/v0/web/platform_pb');

module.exports = {
  v0: {
    ...coreMessages,
    ...platformMessages,
    CorePromiseClient,
    PlatformPromiseClient,
  },
};
