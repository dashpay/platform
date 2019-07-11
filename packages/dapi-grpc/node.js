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

const loadPackageDefinition = require('./src/loadPackageDefinition');
const jsonToProtobufFactory = require('./src/converters/jsonToProtobufFactory');
const protobufToJsonFactory = require('./src/converters/protobufToJsonFactory');

module.exports = Object.assign({
  CorePromiseClient,
  TransactionsFilterStreamPromiseClient,
  utils: {
    loadPackageDefinition,
    jsonToProtobufFactory,
    protobufToJsonFactory,
  },
  pbjs: Object.assign({}, pbjsCoreMessages, pbjsTransactionsFilterStreamMessages),
}, protocCoreMessages, protocTransactionsFilterStreamMessages);
