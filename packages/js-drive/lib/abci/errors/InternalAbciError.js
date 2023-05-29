const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const AbstractAbciError = require('./AbstractAbciError');

class InternalAbciError extends AbstractAbciError {
  /**
   *
   * @param {Error} error
   * @param {Object} [data]
   */
  constructor(error, data = {}) {
    super(grpcErrorCodes.INTERNAL, 'Internal error', data);

    this.error = error;
  }

  /**
   * @returns {Error}
   */
  getError() {
    return this.error;
  }
}

module.exports = InternalAbciError;
