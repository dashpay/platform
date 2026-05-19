import grpcErrorCodes from '@dashevo/grpc-common/lib/server/error/GrpcErrorCodes.js';

import RetriableResponseError from '../../errors/response/RetriableResponseError.js';

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

export default TimeoutError;
