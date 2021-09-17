const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const ResponseError = require('../../errors/response/ResponseError');

class NotFoundError extends ResponseError {
  /**
   *
   * @param {string} message
   * @param {object} data
   * @param {DAPIAddress} dapiAddress
   */
  constructor(message, data, dapiAddress) {
    super(grpcErrorCodes.NOT_FOUND, message, data, dapiAddress);
  }
}

module.exports = NotFoundError;
