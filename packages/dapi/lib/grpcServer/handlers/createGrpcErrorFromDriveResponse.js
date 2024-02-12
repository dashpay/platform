const cbor = require('cbor');

const {
  server: {
    error: {
      InternalGrpcError,
      InvalidArgumentGrpcError,
      DeadlineExceededGrpcError,
      ResourceExhaustedGrpcError,
      NotFoundGrpcError,
      FailedPreconditionGrpcError,
      UnavailableGrpcError,
      GrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const { default: loadWasmDpp, deserializeConsensusError } = require('@dashevo/wasm-dpp');

const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const AlreadyExistsGrpcError = require('@dashevo/grpc-common/lib/server/error/AlreadyExistsGrpcError');
const logger = require('../../logger');

/**
 * @param {Object} data
 * @returns {{"drive-error-data-bin": Buffer}||{}}
 */
function createRawMetadata(data) {
  if (Object.keys(data).length === 0) {
    return {};
  }

  return {
    'drive-error-data-bin': cbor.encode(data),
  };
}

const COMMON_ERROR_CLASSES = {
  [GrpcErrorCodes.INVALID_ARGUMENT]: InvalidArgumentGrpcError,
  [GrpcErrorCodes.DEADLINE_EXCEEDED]: DeadlineExceededGrpcError,
  [GrpcErrorCodes.NOT_FOUND]: NotFoundGrpcError,
  [GrpcErrorCodes.ALREADY_EXISTS]: AlreadyExistsGrpcError,
  [GrpcErrorCodes.RESOURCE_EXHAUSTED]: ResourceExhaustedGrpcError,
  [GrpcErrorCodes.FAILED_PRECONDITION]: FailedPreconditionGrpcError,
  [GrpcErrorCodes.UNAVAILABLE]: UnavailableGrpcError,
};

/**
 * @typedef createGrpcErrorFromDriveResponse
 * @param {number} code
 * @param {string} info
 * @return {GrpcError}
 */
async function createGrpcErrorFromDriveResponse(code, info) {
  await loadWasmDpp();

  if (code === undefined) {
    return new InternalGrpcError(new Error('Drive’s error code is empty'));
  }

  const decodedInfo = info ? cbor.decode(Buffer.from(info, 'base64')) : { };

  // eslint-disable-next-line prefer-destructuring
  const message = decodedInfo.message;
  const data = decodedInfo.data || {};

  // gRPC error codes
  if (code <= 16) {
    const CommonErrorClass = COMMON_ERROR_CLASSES[code.toString()];
    if (CommonErrorClass) {
      return new CommonErrorClass(
        message,
        createRawMetadata(data),
      );
    }

    // TODO(rs-drive-abci): revisit.
    //   Rust does not provide stack trace in case of an error.
    //   It is possible however to use Backtrace crate to report stack.
    //   Decide whether it worth using Backtrace in rs-drive-abci queries
    //   and remove if not needed
    // Restore stack for internal error
    if (code === GrpcErrorCodes.INTERNAL) {
      const error = new Error(message);

      // in case of verbose internal error
      if (data.stack) {
        error.stack = data.stack;

        delete data.stack;
      }

      return new InternalGrpcError(error, createRawMetadata(data));
    }

    return new GrpcError(
      code,
      message,
      createRawMetadata(data),
    );
  }

  // Undefined Drive and DAPI errors
  if (code >= 17 && code < 1000) {
    return new GrpcError(
      GrpcErrorCodes.UNKNOWN,
      message,
      createRawMetadata(data),
    );
  }

  // DPP errors
  if (code >= 1000 && code < 5000) {
    let consensusError;
    try {
      consensusError = deserializeConsensusError(data.serializedError || []);
    } catch (e) {
      logger.error({
        err: e,
        data: data.serializedError,
        code,
      }, `Failed to deserialize consensus error with code ${code}: ${e.message}`);

      throw e;
    }

    // Basic
    if (code >= 1000 && code < 2000) {
      return new InvalidArgumentGrpcError(
        consensusError.message,
        { code, ...createRawMetadata(data) },
      );
    }

    // Signature
    if (code >= 2000 && code < 3000) {
      return new GrpcError(
        GrpcErrorCodes.UNAUTHENTICATED,
        consensusError.message,
        { code, ...createRawMetadata(data) },
      );
    }

    // Fee
    if (code >= 3000 && code < 4000) {
      return new FailedPreconditionGrpcError(
        consensusError.message,
        { code, ...createRawMetadata(data) },
      );
    }

    // State
    if (code >= 4000 && code < 5000) {
      return new InvalidArgumentGrpcError(
        consensusError.message,
        { code, ...createRawMetadata(data) },
      );
    }
  }

  return new InternalGrpcError(new Error(`Unknown Drive’s error code: ${code}`));
}

module.exports = createGrpcErrorFromDriveResponse;
