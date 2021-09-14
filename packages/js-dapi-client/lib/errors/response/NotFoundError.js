const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const ResponseError = require('./ResponseError');

class NotFoundError extends ResponseError {
  /**
   *
   * @param {string} message
   * @param {object} metadata
   * @param {DAPIAddress} dapiAddress
   */
  constructor(message, metadata, dapiAddress) {
    super(grpcErrorCodes.NOT_FOUND, message, metadata, dapiAddress);
  }
}

module.exports = NotFoundError;
