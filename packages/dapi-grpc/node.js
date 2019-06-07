const CorePromiseClient = require('./clients/nodejs/CorePromiseClient');
const TransactionsFilterStreamPromiseClient = require('./clients/nodejs/TransactionsFilterStreamPromiseClient');
const coreMessages = require('./clients/nodejs/core_pb');
const transactionsFilterStreamMessages = require('./clients/nodejs/transactions_filter_stream_pb');
const loadPackageDefinition = require('./src/loadPackageDefinition');

module.exports = Object.assign({
  CorePromiseClient,
  TransactionsFilterStreamPromiseClient,
  loadPackageDefinition,
}, coreMessages, transactionsFilterStreamMessages);
