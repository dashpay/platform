const core = require('./clients/web/core_grpc_web_pb');
const transactionsFilterStream = require('./clients/web/transactions_filter_stream_grpc_web_pb');
const platform = require('./clients/web/platform_grpc_web_pb');

module.exports = Object.assign(
  { },
  core,
  transactionsFilterStream,
  platform,
);
