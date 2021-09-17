const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const RetriableResponseError = require('../../errors/response/RetriableResponseError');

class TimeoutError extends RetriableResponseError {
  /**
   * @param {string} message
   * @param {object} data
   * @param {DAPIAddress} dapiAddress
   */
  constructor(message, data, dapiAddress) {
    super(grpcErrorCodes.DEADLINE_EXCEEDED, message, data, dapiAddress);
  }
}

module.exports = TimeoutError;
