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
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const AlreadyExistsGrpcError = require('@dashevo/grpc-common/lib/server/error/AlreadyExistsGrpcError');
const createConsensusError = require('@dashevo/dpp/lib/errors/consensus/createConsensusError');

/**
 * @typedef createGrpcErrorFromDriveResponse
 * @param {number} code
 * @param {string} info
 * @return {GrpcError}
 */
function createGrpcErrorFromDriveResponse(code, info) {
  if (code === undefined) {
    return new InternalGrpcError(new Error('Drive’s error code is empty'));
  }

  const decodedInfo = info ? cbor.decode(Buffer.from(info, 'base64')) : undefined;

  // eslint-disable-next-line default-case
  switch (code) {
    case GrpcErrorCodes.INVALID_ARGUMENT:
      return new InvalidArgumentGrpcError(decodedInfo.message, decodedInfo.metadata);
    case GrpcErrorCodes.DEADLINE_EXCEEDED:
      return new DeadlineExceededGrpcError(decodedInfo.message, decodedInfo.metadata);
    case GrpcErrorCodes.NOT_FOUND:
      return new NotFoundGrpcError(decodedInfo.message, decodedInfo.metadata);
    case GrpcErrorCodes.ALREADY_EXISTS:
      return new AlreadyExistsGrpcError(decodedInfo.message, decodedInfo.metadata);
    case GrpcErrorCodes.RESOURCE_EXHAUSTED:
      return new ResourceExhaustedGrpcError(decodedInfo.message, decodedInfo.metadata);
    case GrpcErrorCodes.FAILED_PRECONDITION:
      return new FailedPreconditionGrpcError(decodedInfo.message, decodedInfo.metadata);
    case GrpcErrorCodes.INTERNAL: {
      const error = new Error(decodedInfo.message);
      error.stack = decodedInfo.metadata.stack;

      delete decodedInfo.metadata.stack;

      return new InternalGrpcError(error, decodedInfo.metadata);
    }
    case GrpcErrorCodes.UNAVAILABLE:
      return new UnavailableGrpcError(decodedInfo.message, decodedInfo.metadata);
    case GrpcErrorCodes.CANCELLED:
    case GrpcErrorCodes.UNKNOWN:
    case GrpcErrorCodes.UNAUTHENTICATED:
    case GrpcErrorCodes.DATA_LOSS:
    case GrpcErrorCodes.UNIMPLEMENTED:
    case GrpcErrorCodes.OUT_OF_RANGE:
    case GrpcErrorCodes.ABORTED:
    case GrpcErrorCodes.PERMISSION_DENIED:
      return new GrpcError(code, decodedInfo.message, decodedInfo.metadata);
  }

  if (code >= 17 && code < 1000) {
    return new GrpcError(GrpcErrorCodes.UNKNOWN, decodedInfo.message, decodedInfo.metadata);
  }

  if (code >= 1000 && code < 5000) {
    const consensusError = createConsensusError(code, decodedInfo || []);

    // Basic
    if (code >= 1000 && code < 2000) {
      return new InvalidArgumentGrpcError(consensusError.message, { code });
    }

    // Signature
    if (code >= 2000 && code < 3000) {
      return new GrpcError(GrpcErrorCodes.UNAUTHENTICATED, consensusError.message, { code });
    }

    // Fee
    if (code >= 3000 && code < 4000) {
      return new FailedPreconditionGrpcError(consensusError.message, { code });
    }

    // State
    if (code >= 4000 && code < 5000) {
      return new InvalidArgumentGrpcError(consensusError.message, { code });
    }
  }

  return new InternalGrpcError(new Error(`Unknown Drive’s error code: ${code}`));
}

module.exports = createGrpcErrorFromDriveResponse;
