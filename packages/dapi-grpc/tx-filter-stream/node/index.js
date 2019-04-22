const TransactionsFilterStreamClient = require('./TransactionsFilterStreamClient');
const loadPackageDefinition = require('./loadPackageDefinition');
const messages = require('./tx_filter_stream_pb');

module.exports = Object.assign({
  TransactionsFilterStreamClient,
  loadPackageDefinition,
}, messages);
