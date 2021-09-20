const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const AbstractAbciError = require('./AbstractAbciError');

class NotFoundAbciError extends AbstractAbciError {
  /**
   *
   * @param {string} message
   * @param {Object} [data]
   */
  constructor(message, data = {}) {
    super(grpcErrorCodes.NOT_FOUND, message, data);
  }
}

module.exports = NotFoundAbciError;
