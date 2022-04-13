const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const AbstractAbciError = require('./AbstractAbciError');

class UnimplementedAbciError extends AbstractAbciError {
  /**
   *
   * @param {string} message
   * @param {Object} [data]
   */
  constructor(message, data = {}) {
    super(grpcErrorCodes.UNIMPLEMENTED, message, data);
  }
}

module.exports = UnimplementedAbciError;
