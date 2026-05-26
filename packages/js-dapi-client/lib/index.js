const DAPIClient = require('./DAPIClient');

const NotFoundError = require('./transport/GrpcTransport/errors/NotFoundError');
const BlockHeadersProvider = require('./BlockHeadersProvider/BlockHeadersProvider');
const bytes = require('./utils/bytes');

DAPIClient.Errors = {
  NotFoundError,
};

DAPIClient.BlockHeadersProvider = BlockHeadersProvider;
DAPIClient.bytes = bytes;

module.exports = DAPIClient;
