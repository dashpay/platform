const { Metadata, parseMetadata } = require('@dashevo/dapi-grpc');
const GrpcError = require('@dashevo/grpc-common/lib/server/error/GrpcError');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const cbor = require('cbor');
const SerializedObjectParsingError = require('@dashevo/dpp/lib/errors/consensus/basic/decode/SerializedObjectParsingError');
const createGrpcTransportError = require('../../../../lib/transport/GrpcTransport/createGrpcTransportError');
const DAPIAddress = require('../../../../lib/dapiAddressProvider/DAPIAddress');
const NotFoundError = require('../../../../lib/transport/GrpcTransport/errors/NotFoundError');
const InvalidRequestError = require('../../../../lib/transport/errors/response/InvalidRequestError');
const InternalServerError = require('../../../../lib/transport/GrpcTransport/errors/InternalServerError');
const ServerError = require('../../../../lib/transport/errors/response/ServerError');
const InvalidRequestDPPError = require('../../../../lib/transport/errors/response/InvalidRequestDPPError');
const ResponseError = require('../../../../lib/transport/errors/response/ResponseError');

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
    // grpc-js expects Buffer
    let driveErrorDataBin = cbor.encode(errorData);

    // and grpc-web expects base64 string
    if (typeof window !== 'undefined') {
      driveErrorDataBin = driveErrorDataBin.toString('base64');
    }

    metadata.set('drive-error-data-bin', driveErrorDataBin);
  });

  it('should return NotFoundError', () => {
    const grpcError = new GrpcError(
      GrpcErrorCodes.NOT_FOUND,
      'Not found',
    );
    grpcError.metadata = metadata;

    const error = createGrpcTransportError(
      grpcError,
      dapiAddress,
    );

    expect(error).to.be.an.instanceOf(NotFoundError);
    expect(error.message).to.equal(grpcError.message);
    expect(error.getCode()).to.equal(GrpcErrorCodes.NOT_FOUND);
    expect(error.getDAPIAddress()).to.deep.equal(dapiAddress);
    expect(error.getData()).to.deep.equal(errorData);
  });

  it('should get code from metadata', () => {
    metadata.set('code', GrpcErrorCodes.INVALID_ARGUMENT);

    const grpcError = new GrpcError(
      GrpcErrorCodes.NOT_FOUND,
      'Not found',
    );

    grpcError.metadata = metadata;

    const error = createGrpcTransportError(
      grpcError,
      dapiAddress,
    );

    expect(error).to.be.an.instanceOf(InvalidRequestError);
    expect(error.message).to.equal(grpcError.message);
    expect(error.getCode()).to.equal(GrpcErrorCodes.INVALID_ARGUMENT);
    expect(error.getDAPIAddress()).to.deep.equal(dapiAddress);
    expect(error.getData()).to.deep.equal(errorData);
  });

  it('should return InvalidRequestError', () => {
    const grpcError = new GrpcError(
      GrpcErrorCodes.INVALID_ARGUMENT,
      'Invalid arguments',
    );
    grpcError.metadata = metadata;

    const error = createGrpcTransportError(
      grpcError,
      dapiAddress,
    );

    expect(error).to.be.an.instanceOf(InvalidRequestError);
    expect(error.message).to.equal(grpcError.message);
    expect(error.getCode()).to.equal(GrpcErrorCodes.INVALID_ARGUMENT);
    expect(error.getDAPIAddress()).to.deep.equal(dapiAddress);
    expect(error.getData()).to.deep.equal(errorData);
  });

  it('should return InternalServerError with stack', () => {
    const errorWithStack = new Error('Some error');
    const grpcError = new GrpcError(
      GrpcErrorCodes.INTERNAL,
      'Internal error',
    );

    // grpc-js expects Buffer
    let stackBin = cbor.encode(errorWithStack.stack);

    // and grpc-web expects string
    if (typeof window !== 'undefined') {
      stackBin = stackBin.toString('base64');
    }
    metadata.set('stack-bin', stackBin);

    grpcError.metadata = metadata;

    const error = createGrpcTransportError(
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

  it('should return ServerError', () => {
    const grpcError = new GrpcError(
      GrpcErrorCodes.UNAVAILABLE,
      'Unavailable',
    );
    grpcError.metadata = metadata;

    const error = createGrpcTransportError(
      grpcError,
      dapiAddress,
    );

    expect(error).to.be.an.instanceOf(ServerError);
    expect(error.message).to.equal(grpcError.message);
    expect(error.getCode()).to.equal(GrpcErrorCodes.UNAVAILABLE);
    expect(error.getDAPIAddress()).to.deep.equal(dapiAddress);
    expect(error.getData()).to.deep.equal(errorData);
  });

  it('should return InvalidRequestDPPError', () => {
    const constructorArguments = ['arguments'];

    // grpc-js expects Buffer
    let driveErrorDataBin = cbor.encode({
      arguments: constructorArguments,
      ...errorData,
    });

    // and grpc-web expects string
    if (typeof window !== 'undefined') {
      driveErrorDataBin = driveErrorDataBin.toString('base64');
    }
    metadata.set('drive-error-data-bin', driveErrorDataBin);

    const grpcError = new GrpcError(
      1001,
      'Parsing error',
    );
    grpcError.metadata = metadata;

    const error = createGrpcTransportError(
      grpcError,
      dapiAddress,
    );

    expect(error).to.be.an.instanceOf(InvalidRequestDPPError);

    expect(error.getCode()).to.equal(grpcError.code);
    expect(error.getDAPIAddress()).to.deep.equal(dapiAddress);
    expect(error.getData()).to.deep.equal(errorData);

    const consensusError = error.getConsensusError();

    expect(consensusError).to.be.an.instanceOf(SerializedObjectParsingError);
    expect(consensusError.getConstructorArguments()).to.deep.equal(constructorArguments);
  });

  it('should return ResponseError', () => {
    const grpcError = new GrpcError(
      6000,
      'Unknown error',
    );
    grpcError.metadata = metadata;

    const error = createGrpcTransportError(
      grpcError,
      dapiAddress,
    );

    expect(error).to.be.an.instanceOf(ResponseError);
    expect(error.message).to.equal(grpcError.message);
    expect(error.getCode()).to.equal(grpcError.code);
    expect(error.getDAPIAddress()).to.deep.equal(dapiAddress);
    expect(error.getData()).to.deep.equal(errorData);
  });

  it('should handle plain object metadata', () => {
    const objectMetadata = parseMetadata(metadata);
    const grpcError = new GrpcError(
      GrpcErrorCodes.NOT_FOUND,
      'Not found',
    );
    grpcError.metadata = objectMetadata;

    const error = createGrpcTransportError(
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
