const cbor = require('cbor');

const createConsensusError = require('@dashevo/dpp/lib/errors/consensus/createConsensusError');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const NotFoundError = require('./errors/NotFoundError');
const TimeoutError = require('./errors/TimeoutError');
const ResponseError = require('../errors/response/ResponseError');
const ServerError = require('../errors/response/ServerError');
const InvalidRequestError = require('../errors/response/InvalidRequestError');
const InvalidRequestDPPError = require('../errors/response/InvalidRequestDPPError');
const InternalServerError = require('./errors/InternalServerError');

const INVALID_REQUEST_CODES = [
  GrpcErrorCodes.INVALID_ARGUMENT,
  GrpcErrorCodes.FAILED_PRECONDITION,
  GrpcErrorCodes.ALREADY_EXISTS,
  GrpcErrorCodes.UNAUTHENTICATED,
  GrpcErrorCodes.OUT_OF_RANGE,
  GrpcErrorCodes.PERMISSION_DENIED,
];

const SERVER_ERROR_CODES = [
  GrpcErrorCodes.RESOURCE_EXHAUSTED,
  GrpcErrorCodes.UNAVAILABLE,
  GrpcErrorCodes.CANCELLED,
  GrpcErrorCodes.UNKNOWN,
  GrpcErrorCodes.DATA_LOSS,
  GrpcErrorCodes.UNIMPLEMENTED,
  GrpcErrorCodes.ABORTED,
  GrpcErrorCodes.INTERNAL,
];

const errorClasses = {
  [GrpcErrorCodes.NOT_FOUND]: NotFoundError,
  [GrpcErrorCodes.DEADLINE_EXCEEDED]: TimeoutError,
};

/**
 * @typedef {createGrpcTransportError}
 * @param {Error} grpcError
 * @param {DAPIAddress} dapiAddress
 * @returns {ResponseError}
 */
function createGrpcTransportError(grpcError, dapiAddress) {
  // Extract error code and data
  let data = {};
  let { code } = grpcError;

  if (grpcError.metadata) {
    const cboredMetaData = grpcError.metadata.get('drive-error-data-bin');
    if (cboredMetaData && cboredMetaData.length > 0) {
      data = cbor.decode(cboredMetaData[0]);
    }

    // since gRPC doesn't allow to use custom error codes
    // DAPI pass them as a part of metadata
    const metaCode = grpcError.metadata.get('code');
    if (metaCode && metaCode.length > 0) {
      [code] = metaCode;
    }
  }

  // Specialized classes
  const ErrorClass = errorClasses[code.toString()];

  if (ErrorClass) {
    return new ErrorClass(
      grpcError.message,
      data,
      dapiAddress,
    );
  }

  // Invalid request
  if (INVALID_REQUEST_CODES.includes(code)) {
    return new InvalidRequestError(
      code,
      grpcError.message,
      data,
      dapiAddress,
    );
  }

  if (code === GrpcErrorCodes.INTERNAL) {
    if (grpcError.metadata) {
      const metaStack = grpcError.metadata.get('stack-bin');
      if (metaStack && metaStack.length > 0) {
        data.stack = cbor.decode(metaStack[0]);
      }
    }

    return new InternalServerError(
      code,
      grpcError.message,
      data,
      dapiAddress,
    );
  }

  // Server error
  if (SERVER_ERROR_CODES.includes(code)) {
    return new ServerError(
      code,
      grpcError.message,
      data,
      dapiAddress,
    );
  }

  // DPP consensus errors
  if (code >= 1000 && code < 5000) {
    const consensusError = createConsensusError(code, data.arguments || []);

    delete data.arguments;

    return new InvalidRequestDPPError(consensusError, data, dapiAddress);
  }

  return new ResponseError(
    code,
    grpcError.message,
    data,
    dapiAddress,
  );
}

module.exports = createGrpcTransportError;
