const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const GrpcError = require('@dashevo/grpc-common/lib/server/error/GrpcError');
const cbor = require('cbor');
const InternalGrpcError = require('@dashevo/grpc-common/lib/server/error/InternalGrpcError');
const InvalidArgumentGrpcError = require('@dashevo/grpc-common/lib/server/error/InvalidArgumentGrpcError');
const FailedPreconditionGrpcError = require('@dashevo/grpc-common/lib/server/error/FailedPreconditionGrpcError');
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const createGrpcErrorFromDriveResponse = require(
  '../../../../lib/grpcServer/handlers/createGrpcErrorFromDriveResponse',
);

describe('createGrpcErrorFromDriveResponse', () => {
  let message;
  let info;
  let encodedInfo;

  beforeEach(() => {
    message = 'message';
    info = {
      message,
      metadata: {
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
      it(`should throw ${codeClass} if response code is ${code}`, () => {
        const error = createGrpcErrorFromDriveResponse(code, encodedInfo);

        let messageToCheck = message;

        if (code === GrpcErrorCodes.FAILED_PRECONDITION) {
          messageToCheck = `Failed precondition: ${messageToCheck}`;
        }

        expect(error).to.be.an.instanceOf(GrpcError);
        expect(error.getMessage()).to.equal(messageToCheck);
        expect(error.getCode()).to.equal(code);
        expect(error.getRawMetadata()).to.deep.equal(info.metadata);
      });
    });

  it('should throw GrpcError if error code = 17', () => {
    const error = createGrpcErrorFromDriveResponse(17, encodedInfo);

    expect(error).to.be.an.instanceOf(GrpcError);
    expect(error.getMessage()).to.equal(message);
    expect(error.getCode()).to.equal(GrpcErrorCodes.UNKNOWN);
    expect(error.getRawMetadata()).to.deep.equal(info.metadata);
  });

  it('should throw ConsensusError if error code = 1000', () => {
    const error = createGrpcErrorFromDriveResponse(1000);

    expect(error).to.be.an.instanceOf(InvalidArgumentGrpcError);
    expect(error.getRawMetadata()).to.deep.equal({ code: 1000 });
  });

  it('should throw ConsensusError if error code = 2000', () => {
    const id = generateRandomIdentifier();

    const error = createGrpcErrorFromDriveResponse(
      2000,
      cbor.encode([id]).toString('base64'),
    );

    expect(error).to.be.an.instanceOf(GrpcError);
    expect(error.getCode()).to.equal(GrpcErrorCodes.UNAUTHENTICATED);
    expect(error.getRawMetadata()).to.deep.equal({ code: 2000 });
  });

  it('should throw ConsensusError if error code = 3000', () => {
    const error = createGrpcErrorFromDriveResponse(3000, cbor.encode([20, 10]).toString('base64'));

    expect(error).to.be.an.instanceOf(FailedPreconditionGrpcError);
    expect(error.getRawMetadata()).to.deep.equal({ code: 3000 });
  });

  it('should throw ConsensusError if error code = 4000', () => {
    const dataContractId = generateRandomIdentifier();

    const error = createGrpcErrorFromDriveResponse(4000, cbor.encode([dataContractId]).toString('base64'));

    expect(error).to.be.an.instanceOf(InvalidArgumentGrpcError);
    expect(error.getRawMetadata()).to.deep.equal({ code: 4000 });
  });

  it('should throw Unknown error code >= 5000', () => {
    const error = createGrpcErrorFromDriveResponse(5000, encodedInfo);

    expect(error).to.be.an.instanceOf(GrpcError);
    expect(error.getMessage()).to.equal('Internal error');
    expect(error.getError().message).to.deep.equal('Unknown Drive’s error code: 5000');
  });

  it('should return InternalGrpcError if codes is undefined', () => {
    const error = createGrpcErrorFromDriveResponse();

    expect(error).to.be.an.instanceOf(InternalGrpcError);
    expect(error.getMessage()).to.equal('Internal error');
    expect(error.getError().message).to.deep.equal('Drive’s error code is empty');
  });

  it('should return InternalGrpcError if code = 13', () => {
    const errorInfo = {
      message,
      metadata: {
        ...info.metadata,
        stack: {
          data: 'stack info',
        },
      },
    };

    const error = createGrpcErrorFromDriveResponse(
      GrpcErrorCodes.INTERNAL,
      cbor.encode(errorInfo).toString('base64'),
    );

    expect(error).to.be.an.instanceOf(InternalGrpcError);
    expect(error.getMessage()).to.equal('Internal error');
    expect(error.getCode()).to.equal(GrpcErrorCodes.INTERNAL);
    expect(error.getError().message).to.deep.equal(message);
    expect(error.getError().stack).to.deep.equal(errorInfo.metadata.stack);
    expect(error.getRawMetadata()).to.deep.equal(info.metadata);
  });
});
