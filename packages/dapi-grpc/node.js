const CorePromiseClient = require('./core/node/CorePromiseClient');
const TransactionsFilterStreamPromiseClient = require('./transactions-filter-stream/node/TransactionsFilterStreamPromiseClient');
const coreMessages = require('./core/node/core_pb');
const transactionsFilterStreamMessages = require('./transactions-filter-stream/node/transactions_filter_stream_pb');
const loadPackageDefinition = require('./src/loadPackageDefinition');

module.exports = Object.assign({
  CorePromiseClient,
  TransactionsFilterStreamPromiseClient,
  loadPackageDefinition,
}, coreMessages, transactionsFilterStreamMessages);
