const cbor = require('cbor');

const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const { parseMetadata } = require('@dashevo/dapi-grpc');

const {
  deserializeConsensusError,
} = require('@dashevo/wasm-dpp');
const { default: loadWasmDpp } = require('@dashevo/wasm-dpp');
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
async function createGrpcTransportError(grpcError, dapiAddress) {
  await loadWasmDpp();

  // Extract error code and data
  let data = {};
  let { code } = grpcError;

  const message = grpcError.details || grpcError.message;

  const metadata = parseMetadata(grpcError.metadata) || {};

  // Error data
  const driveErrorData = metadata['drive-error-data-bin'];
  if (driveErrorData) {
    const encodedData = Buffer.from(driveErrorData, 'base64');
    data = cbor.decode(encodedData);
  }

  // Error code
  const driveErrorCode = metadata.code;
  if (driveErrorCode) {
    code = Number(driveErrorCode);
  }

  // Error stack
  const driveErrorStack = metadata['stack-bin'];
  if (driveErrorStack) {
    const encodedStack = Buffer.from(driveErrorStack, 'base64');
    data.stack = cbor.decode(encodedStack);
  }

  // Specialized classes
  const ErrorClass = errorClasses[code.toString()];

  if (ErrorClass) {
    return new ErrorClass(
      message,
      data,
      dapiAddress,
    );
  }

  // Invalid request
  if (INVALID_REQUEST_CODES.includes(code)) {
    return new InvalidRequestError(
      code,
      message,
      data,
      dapiAddress,
    );
  }

  if (code === GrpcErrorCodes.INTERNAL) {
    return new InternalServerError(
      code,
      message,
      data,
      dapiAddress,
    );
  }

  // Server error
  if (SERVER_ERROR_CODES.includes(code)) {
    return new ServerError(
      code,
      message,
      data,
      dapiAddress,
    );
  }

  // DPP consensus errors
  if (code >= 1000 && code < 5000) {
    const consensusError = deserializeConsensusError(data.serializedError || []);

    delete data.serializedError;

    return new InvalidRequestDPPError(consensusError, data, dapiAddress);
  }

  return new ResponseError(
    code,
    message,
    data,
    dapiAddress,
  );
}

module.exports = createGrpcTransportError;
