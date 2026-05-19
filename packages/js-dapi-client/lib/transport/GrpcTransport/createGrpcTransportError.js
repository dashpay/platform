import cbor from 'cbor';

import GrpcErrorCodes from '@dashevo/grpc-common/lib/server/error/GrpcErrorCodes.js';
import dapiGrpc from '@dashevo/dapi-grpc';

import wasmDpp from '@dashevo/wasm-dpp';
import NotFoundError from './errors/NotFoundError.js';
import TimeoutError from './errors/TimeoutError.js';
import ResponseError from '../errors/response/ResponseError.js';
import ServerError from '../errors/response/ServerError.js';
import InvalidRequestError from '../errors/response/InvalidRequestError.js';
import InvalidRequestDPPError from '../errors/response/InvalidRequestDPPError.js';
import InternalServerError from './errors/InternalServerError.js';
import { base64ToBytes } from '../../utils/bytes.js';

const { parseMetadata } = dapiGrpc;
const { deserializeConsensusError, default: loadWasmDpp } = wasmDpp;

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

  // Error data — grpc-js passes raw bytes; grpc-web passes a base64 string.
  // Original Buffer.from(x, 'base64') silently passed through bytes; reproduce that.
  const driveErrorData = metadata['drive-error-data-bin'];
  if (driveErrorData) {
    const encodedData = typeof driveErrorData === 'string'
      ? base64ToBytes(driveErrorData)
      : new Uint8Array(driveErrorData);
    data = cbor.decode(encodedData);
  }

  // Error code
  const driveErrorCode = metadata.code;
  if (driveErrorCode) {
    code = Number(driveErrorCode);
  }

  // Error stack — same dual-format handling as driveErrorData above.
  const driveErrorStack = metadata['stack-bin'];
  if (driveErrorStack) {
    const encodedStack = typeof driveErrorStack === 'string'
      ? base64ToBytes(driveErrorStack)
      : new Uint8Array(driveErrorStack);
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
  if (code >= 10000 && code < 50000) {
    const consensusErrorString = metadata['dash-serialized-consensus-error-bin'];
    if (!consensusErrorString) {
      throw new Error(`Can't deserialize consensus error ${code}: serialized data is missing`);
    }

    // grpc-js passes raw bytes; grpc-web passes a base64 string. Handle both.
    const consensusErrorBytes = typeof consensusErrorString === 'string'
      ? base64ToBytes(consensusErrorString)
      : new Uint8Array(consensusErrorString);
    const consensusError = deserializeConsensusError(consensusErrorBytes);

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

export default createGrpcTransportError;
