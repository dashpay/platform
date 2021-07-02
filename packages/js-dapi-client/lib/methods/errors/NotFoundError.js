const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const ResponseError = require('./ResponseError');

class NotFoundError extends ResponseError {
  constructor(message) {
    super(grpcErrorCodes.NOT_FOUND, message);
  }
}

module.exports = NotFoundError;
