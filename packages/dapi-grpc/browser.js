const core = require('./clients/core/v0/web/core_grpc_web_pb');
const platform = require('./clients/platform/v0/web/platform_grpc_web_pb');

module.exports = {
  v0: Object.assign(
    { },
    core,
    platform,
  ),
};
