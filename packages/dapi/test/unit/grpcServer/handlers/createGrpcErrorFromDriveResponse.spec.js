const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const GrpcError = require('@dashevo/grpc-common/lib/server/error/GrpcError');
const cbor = require('cbor');
const InternalGrpcError = require('@dashevo/grpc-common/lib/server/error/InternalGrpcError');
const InvalidArgumentGrpcError = require('@dashevo/grpc-common/lib/server/error/InvalidArgumentGrpcError');
const FailedPreconditionGrpcError = require('@dashevo/grpc-common/lib/server/error/FailedPreconditionGrpcError');
const generateRandomIdentifierAsync = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');

const {
  default: loadWasmDpp, ProtocolVersionParsingError,
  IdentityNotFoundError,
  BalanceIsNotEnoughError,
  DataContractAlreadyPresentError,
} = require('@dashevo/wasm-dpp');
const createGrpcErrorFromDriveResponse = require(
  '../../../../lib/grpcServer/handlers/createGrpcErrorFromDriveResponse',
);

describe('createGrpcErrorFromDriveResponse', () => {
  let message;
  let info;
  let encodedInfo;

  before(async () => {
    await loadWasmDpp();
  });

  beforeEach(() => {
    message = 'message';
    info = {
      message,
      data: {
        error: 'some data',
      },
    };

    encodedInfo = cbor.encode(info).toString('base64');
  });

  Object.entries(GrpcErrorCodes)
    // We have special tests below for these error codes
    .filter(([, code]) => (
      ![GrpcErrorCodes.VERSION_MISMATCH, GrpcErrorCodes.INTERNAL].includes(code)
    ))
    .forEach(([codeClass, code]) => {
      it(`should throw ${codeClass} if response code is ${code}`, async () => {
        const error = await createGrpcErrorFromDriveResponse(code, encodedInfo);

        expect(error).to.be.an.instanceOf(GrpcError);
        expect(error.getMessage()).to.equal(message);
        expect(error.getCode()).to.equal(code);
        expect(error.getRawMetadata()).to.deep.equal({
          'drive-error-data-bin': cbor.encode(info.data),
        });
      });
    });

  it('should throw GrpcError if error code = 17', async () => {
    const error = await createGrpcErrorFromDriveResponse(17, encodedInfo);

    expect(error).to.be.an.instanceOf(GrpcError);
    expect(error.getMessage()).to.equal(message);
    expect(error.getCode()).to.equal(GrpcErrorCodes.UNKNOWN);
    expect(error.getRawMetadata()).to.deep.equal({
      'drive-error-data-bin': cbor.encode(info.data),
    });
  });

  it('should throw basic consensus error if error code = 10000', async () => {
    const consensusError = new ProtocolVersionParsingError('test');

    const data = { serializedError: consensusError.serialize() };
    info = { data };

    const error = await createGrpcErrorFromDriveResponse(10000, cbor.encode(info).toString('base64'));

    expect(error).to.be.an.instanceOf(InvalidArgumentGrpcError);
    expect(error.message).to.be.equals(consensusError.message);
    expect(error.getRawMetadata()).to.deep.equal({
      code: 10000,
      'drive-error-data-bin': cbor.encode(data),
    });
  });

  it('should throw signature consensus error if error code = 20000', async () => {
    const id = await generateRandomIdentifierAsync();

    const consensusError = new IdentityNotFoundError(id);

    const data = { serializedError: consensusError.serialize() };
    info = { data };

    const error = await createGrpcErrorFromDriveResponse(
      20000,
      cbor.encode(info).toString('base64'),
    );

    expect(error).to.be.an.instanceOf(GrpcError);
    expect(error.message).to.be.equals(consensusError.message);
    expect(error.getCode()).to.equal(GrpcErrorCodes.UNAUTHENTICATED);
    expect(error.getRawMetadata()).to.deep.equal({
      code: 20000,
      'drive-error-data-bin': cbor.encode(data),
    });
  });

  it('should throw fee consensus error if error code = 30000', async () => {
    const consensusError = new BalanceIsNotEnoughError(BigInt(20), BigInt(10));

    const data = { serializedError: consensusError.serialize() };
    info = { data };

    const error = await createGrpcErrorFromDriveResponse(30000, cbor.encode(info).toString('base64'));

    expect(error).to.be.an.instanceOf(FailedPreconditionGrpcError);
    expect(error.getRawMetadata()).to.deep.equal({
      code: 30000,
      'drive-error-data-bin': cbor.encode(data),
    });
  });

  it('should throw state consensus error if error code = 40000', async () => {
    const dataContractId = await generateRandomIdentifierAsync();

    const consensusError = new DataContractAlreadyPresentError(dataContractId);

    const data = { serializedError: consensusError.serialize() };
    info = { data };

    const error = await createGrpcErrorFromDriveResponse(
      40000,
      cbor.encode(info).toString('base64'),
    );

    expect(error).to.be.an.instanceOf(InvalidArgumentGrpcError);
    expect(error.getRawMetadata()).to.deep.equal({
      code: 40000,
      'drive-error-data-bin': cbor.encode(data),
    });
  });

  it('should throw Unknown error code >= 50000', async () => {
    const error = await createGrpcErrorFromDriveResponse(50000, encodedInfo);

    expect(error).to.be.an.instanceOf(GrpcError);
    expect(error.getMessage()).to.equal('Internal error');
    expect(error.getError().message).to.deep.equal('Unknown Drive’s error code: 50000');
  });

  it('should return InternalGrpcError if codes is undefined', async () => {
    const error = await createGrpcErrorFromDriveResponse();

    expect(error).to.be.an.instanceOf(InternalGrpcError);
    expect(error.getMessage()).to.equal('Internal error');
    expect(error.getError().message).to.deep.equal('Drive’s error code is empty');
  });

  it('should return InternalGrpcError if code = 13', async () => {
    const errorInfo = {
      message,
      data: {
        ...info.data,
        stack: 'long \n long \n long \n string',
      },
    };

    const error = await createGrpcErrorFromDriveResponse(
      GrpcErrorCodes.INTERNAL,
      cbor.encode(errorInfo).toString('base64'),
    );

    expect(error).to.be.an.instanceOf(InternalGrpcError);
    expect(error.getMessage()).to.equal('Internal error');
    expect(error.getCode()).to.equal(GrpcErrorCodes.INTERNAL);
    expect(error.getError().message).to.deep.equal(message);
    expect(error.getError().stack).to.deep.equal(errorInfo.data.stack);
    expect(error.getRawMetadata()).to.deep.equal({
      'drive-error-data-bin': cbor.encode(info.data),
    });
  });
});
