const CorePromiseClient = require('./clients/nodejs/CorePromiseClient');
const TransactionsFilterStreamPromiseClient = require('./clients/nodejs/TransactionsFilterStreamPromiseClient');

const coreMessages = require('./clients/nodejs/core_protoc');
const transactionsFilterStreamMessages = require('./clients/nodejs/transactions_filter_stream_protoc');

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
}, coreMessages, transactionsFilterStreamMessages);
