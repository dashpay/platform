import grpcErrorCodes from '@dashevo/grpc-common/lib/server/error/GrpcErrorCodes.js';

import ResponseError from '../../errors/response/ResponseError.js';

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

export default NotFoundError;
