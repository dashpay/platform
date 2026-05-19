import dapiGrpc from '@dashevo/dapi-grpc';
import GrpcError from '@dashevo/grpc-common/lib/server/error/GrpcError.js';
import GrpcErrorCodes from '@dashevo/grpc-common/lib/server/error/GrpcErrorCodes.js';

import wasmDpp from '@dashevo/wasm-dpp';

import cbor from 'cbor';
import createGrpcTransportError from '../../../../lib/transport/GrpcTransport/createGrpcTransportError.js';
import DAPIAddress from '../../../../lib/dapiAddressProvider/DAPIAddress.js';
import NotFoundError from '../../../../lib/transport/GrpcTransport/errors/NotFoundError.js';
import InvalidRequestError from '../../../../lib/transport/errors/response/InvalidRequestError.js';
import InternalServerError from '../../../../lib/transport/GrpcTransport/errors/InternalServerError.js';
import ServerError from '../../../../lib/transport/errors/response/ServerError.js';
import InvalidRequestDPPError from '../../../../lib/transport/errors/response/InvalidRequestDPPError.js';
import ResponseError from '../../../../lib/transport/errors/response/ResponseError.js';
import { bytesToBase64 } from '../../../../lib/utils/bytes.js';

const { Metadata, parseMetadata } = dapiGrpc;
const { ProtocolVersionParsingError } = wasmDpp;

describe('createGrpcTransportError', () => {
  let dapiAddress;
  let errorData;
  let metadata;

  beforeEach(() => {
    dapiAddress = new DAPIAddress('127.0.0.1:3001:3002');
    errorData = {
      errorData: 'some data',
    };

    metadata = new Metadata();
    // grpc-js expects bytes
    let driveErrorDataBin = cbor.encode(errorData);

    // and grpc-web expects base64 string
    // TODO: remove when we switch to single grpc implementation for both Node and Web
    if (typeof window !== 'undefined') {
      driveErrorDataBin = bytesToBase64(driveErrorDataBin);
    }

    metadata.set('drive-error-data-bin', driveErrorDataBin);
  });

  it('should return NotFoundError', async () => {
    const grpcError = new GrpcError(
      GrpcErrorCodes.NOT_FOUND,
      'Not found',
    );
    grpcError.metadata = metadata;

    const error = await createGrpcTransportError(
      grpcError,
      dapiAddress,
    );

    expect(error).to.be.an.instanceOf(NotFoundError);
    expect(error.message).to.equal(grpcError.message);
    expect(error.getCode()).to.equal(GrpcErrorCodes.NOT_FOUND);
    expect(error.getDAPIAddress()).to.deep.equal(dapiAddress);
    expect(error.getData()).to.deep.equal(errorData);
  });

  it('should get code from metadata', async () => {
    metadata.set('code', GrpcErrorCodes.INVALID_ARGUMENT);

    const grpcError = new GrpcError(
      GrpcErrorCodes.NOT_FOUND,
      'Not found',
    );

    grpcError.metadata = metadata;

    const error = await createGrpcTransportError(
      grpcError,
      dapiAddress,
    );

    expect(error).to.be.an.instanceOf(InvalidRequestError);
    expect(error.message).to.equal(grpcError.message);
    expect(error.getCode()).to.equal(GrpcErrorCodes.INVALID_ARGUMENT);
    expect(error.getDAPIAddress()).to.deep.equal(dapiAddress);
    expect(error.getData()).to.deep.equal(errorData);
  });

  it('should return InvalidRequestError', async () => {
    const grpcError = new GrpcError(
      GrpcErrorCodes.INVALID_ARGUMENT,
      'Invalid arguments',
    );
    grpcError.metadata = metadata;

    const error = await createGrpcTransportError(
      grpcError,
      dapiAddress,
    );

    expect(error).to.be.an.instanceOf(InvalidRequestError);
    expect(error.message).to.equal(grpcError.message);
    expect(error.getCode()).to.equal(GrpcErrorCodes.INVALID_ARGUMENT);
    expect(error.getDAPIAddress()).to.deep.equal(dapiAddress);
    expect(error.getData()).to.deep.equal(errorData);
  });

  it('should return InternalServerError with stack', async () => {
    const errorWithStack = new Error('Some error');
    const grpcError = new GrpcError(
      GrpcErrorCodes.INTERNAL,
      'Internal error',
    );

    // grpc-js expects bytes
    let stackBin = cbor.encode(errorWithStack.stack);

    // and grpc-web expects string
    // TODO: remove when we switch to single grpc implementation for both Node and Web
    if (typeof window !== 'undefined') {
      stackBin = bytesToBase64(stackBin);
    }
    metadata.set('stack-bin', stackBin);

    grpcError.metadata = metadata;

    const error = await createGrpcTransportError(
      grpcError,
      dapiAddress,
    );
    expect(error).to.be.an.instanceOf(InternalServerError);
    expect(error.message).to.equal(grpcError.message);
    expect(error.getCode()).to.equal(GrpcErrorCodes.INTERNAL);
    expect(error.getDAPIAddress()).to.deep.equal(dapiAddress);
    expect(error.getData()).to.deep.equal({
      ...errorData,
      stack: errorWithStack.stack,
    });
    expect(error.stack).to.deep.equal(`[REMOTE STACK] ${errorWithStack.stack}`);
  });

  it('should return ServerError', async () => {
    const grpcError = new GrpcError(
      GrpcErrorCodes.UNAVAILABLE,
      'Unavailable',
    );
    grpcError.metadata = metadata;

    const error = await createGrpcTransportError(
      grpcError,
      dapiAddress,
    );

    expect(error).to.be.an.instanceOf(ServerError);
    expect(error.message).to.equal(grpcError.message);
    expect(error.getCode()).to.equal(GrpcErrorCodes.UNAVAILABLE);
    expect(error.getDAPIAddress()).to.deep.equal(dapiAddress);
    expect(error.getData()).to.deep.equal(errorData);
  });

  it('should return InvalidRequestDPPError', async () => {
    // grpc-js expects bytes
    let serializedError = new ProtocolVersionParsingError('test').serialize();

    // and grpc-web expects string
    // TODO: remove when we switch to single grpc implementation for both Node and Web
    if (typeof window !== 'undefined') {
      serializedError = bytesToBase64(serializedError);
    }
    metadata.set('dash-serialized-consensus-error-bin', serializedError);

    const grpcError = new GrpcError(
      10001,
      'Parsing error',
    );
    grpcError.metadata = metadata;

    const error = await createGrpcTransportError(
      grpcError,
      dapiAddress,
    );

    expect(error).to.be.an.instanceOf(InvalidRequestDPPError);

    expect(error.getCode()).to.equal(grpcError.code);
    expect(error.getDAPIAddress()).to.deep.equal(dapiAddress);
    expect(error.getData()).to.deep.equal(errorData);

    const consensusError = error.getConsensusError();

    expect(consensusError).to.be.an.instanceOf(ProtocolVersionParsingError);
  });

  it('should return ResponseError', async () => {
    const grpcError = new GrpcError(
      6000,
      'Unknown error',
    );
    grpcError.metadata = metadata;

    const error = await createGrpcTransportError(
      grpcError,
      dapiAddress,
    );

    expect(error).to.be.an.instanceOf(ResponseError);
    expect(error.message).to.equal(grpcError.message);
    expect(error.getCode()).to.equal(grpcError.code);
    expect(error.getDAPIAddress()).to.deep.equal(dapiAddress);
    expect(error.getData()).to.deep.equal(errorData);
  });

  it('should handle plain object metadata', async () => {
    const objectMetadata = parseMetadata(metadata);
    const grpcError = new GrpcError(
      GrpcErrorCodes.NOT_FOUND,
      'Not found',
    );
    grpcError.metadata = objectMetadata;

    const error = await createGrpcTransportError(
      grpcError,
      dapiAddress,
    );

    expect(error).to.be.an.instanceOf(NotFoundError);
    expect(error.message).to.equal(grpcError.message);
    expect(error.getCode()).to.equal(GrpcErrorCodes.NOT_FOUND);
    expect(error.getDAPIAddress()).to.deep.equal(dapiAddress);
    expect(error.getData()).to.deep.equal(errorData);
  });
});
