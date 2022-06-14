const DAPIClient = require('./DAPIClient');

const NotFoundError = require('./transport/GrpcTransport/errors/NotFoundError');

DAPIClient.Errors = {
  NotFoundError,
};

module.exports = DAPIClient;
