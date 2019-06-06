const core = require('./core/web/core_grpc_web_pb');
const transactionsFilterStream = require('./transactions-filter-stream/web/transactions_filter_stream_grpc_web_pb');

module.exports = Object.assign(
  { },
  core,
  transactionsFilterStream,
);
