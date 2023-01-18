require('../polyfills/fetch-polyfill');

const DAPIClient = require('./DAPIClient');

const NotFoundError = require('./transport/GrpcTransport/errors/NotFoundError');
const BlockHeadersProvider = require('./BlockHeadersProvider/BlockHeadersProvider');

DAPIClient.Errors = {
  NotFoundError,
};

DAPIClient.BlockHeadersProvider = BlockHeadersProvider;

module.exports = DAPIClient;
