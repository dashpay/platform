const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const AbstractAbciError = require('./AbstractAbciError');

class InvalidArgumentAbciError extends AbstractAbciError {
  /**
   *
   * @param {string} message
   * @param {Object} [data]
   */
  constructor(message, data = {}) {
    super(grpcErrorCodes.INVALID_ARGUMENT, message, data);
  }
}

module.exports = InvalidArgumentAbciError;
