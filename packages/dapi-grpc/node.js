const CorePromiseClient = require('./clients/nodejs/CorePromiseClient');
const TransactionsFilterStreamPromiseClient = require('./clients/nodejs/TransactionsFilterStreamPromiseClient');

const protocCoreMessages = require('./clients/nodejs/core_protoc');
const protocTransactionsFilterStreamMessages = require('./clients/nodejs/transactions_filter_stream_protoc');

const {
  org: {
    dash: {
      platform: {
        dapi: pbjsCoreMessages,
      },
    },
  },
} = require('./clients/nodejs/core_pbjs');

const {
  org: {
    dash: {
      platform: {
        dapi: pbjsTransactionsFilterStreamMessages,
      },
    },
  },
} = require('./clients/nodejs/transactions_filter_stream_pbjs');

module.exports = Object.assign({
  CorePromiseClient,
  TransactionsFilterStreamPromiseClient,
  pbjs: Object.assign({}, pbjsCoreMessages, pbjsTransactionsFilterStreamMessages),
}, protocCoreMessages, protocTransactionsFilterStreamMessages);
