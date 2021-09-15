const cbor = require('cbor');
const InternalGrpcError = require('@dashevo/grpc-common/lib/server/error/InternalGrpcError');
const InvalidArgumentGrpcError = require('@dashevo/grpc-common/lib/server/error/InvalidArgumentGrpcError');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const SomeConsensusError = require('@dashevo/dpp/lib/test/mocks/SomeConsensusError');
const wrapInErrorHandlerFactory = require('../../../../lib/abci/errors/wrapInErrorHandlerFactory');
const LoggerMock = require('../../../../lib/test/mock/LoggerMock');
const DPPValidationError = require('../../../../lib/abci/handlers/errors/DPPValidationError');

describe('wrapInErrorHandlerFactory', () => {
  let loggerMock;
  let methodMock;
  let request;
  let handler;
  let wrapInErrorHandler;

  beforeEach(function beforeEach() {
    request = {
      tx: Buffer.alloc(0),
    };

    loggerMock = new LoggerMock(this.sinon);

    wrapInErrorHandler = wrapInErrorHandlerFactory(loggerMock, true);
    methodMock = this.sinon.stub();

    handler = wrapInErrorHandler(
      methodMock,
    );
  });

  it('should throw an internal error if any Error is thrown in handler', async () => {
    const error = new Error('Custom error');

    methodMock.throws(error);

    try {
      await handler(request);

      expect.fail('Internal error must be thrown');
    } catch (e) {
      expect(e).to.equal(error);
    }
  });

  it('should throw en internal error if an InternalAbciError is thrown in handler', async () => {
    const originError = new Error();
    const metadata = { sample: 'data' };
    const error = new InternalGrpcError(originError, metadata);

    methodMock.throws(error);

    try {
      await handler(request);

      expect.fail('Internal error must be thrown');
    } catch (e) {
      expect(e).to.equal(originError);
    }
  });

  it('should respond with internal error code if any Error is thrown in handler and respondWithInternalError enabled', async () => {
    handler = wrapInErrorHandler(
      methodMock, { respondWithInternalError: true },
    );

    const error = new Error('Custom error');

    methodMock.throws(error);

    const response = await handler(request);

    expect(response).to.deep.equal({
      code: GrpcErrorCodes.INTERNAL,
      info: cbor.encode({
        message: 'Internal error',
        metadata: undefined,
      }).toString('base64'),
    });
  });

  it('should respond with internal error code if an InternalAbciError is thrown in handler and respondWithInternalError enabled', async () => {
    handler = wrapInErrorHandler(
      methodMock, { respondWithInternalError: true },
    );

    const data = { sample: 'data' };
    const error = new InternalGrpcError(new Error(), data);

    methodMock.throws(error);

    const response = await handler(request);

    expect(response).to.deep.equal({
      code: error.getCode(),
      info: cbor.encode({
        message: error.getMessage(),
        metadata: error.getRawMetadata(),
      }).toString('base64'),
    });
  });

  it('should respond with invalid argument error if it is thrown in handler', async () => {
    const data = { sample: 'data' };
    const error = new InvalidArgumentGrpcError('test', data);

    methodMock.throws(error);

    const response = await handler(request);

    expect(response).to.deep.equal({
      code: error.getCode(),
      info: cbor.encode({
        message: error.getMessage(),
        metadata: error.getRawMetadata(),
      }).toString('base64'),
    });
  });

  it('should rethrow internal errors in case `rethrowInternalErrors` options is set', async () => {
    const unknownError = new Error('Some internal error indicating a bug');

    methodMock.throws(unknownError);

    handler = wrapInErrorHandler(
      methodMock, { throwNonABCIErrors: true },
    );

    try {
      await handler(request);
      expect.fail('Error was not re-thrown');
    } catch (e) {
      expect(e).to.equal(unknownError);
    }
  });

  it('should respond with verbose error containing message and stack in debug mode', async () => {
    wrapInErrorHandler = wrapInErrorHandlerFactory(loggerMock, false);

    const error = new Error('Custom error');

    methodMock.throws(error);

    handler = wrapInErrorHandler(
      methodMock, { respondWithInternalError: true },
    );

    const response = await handler(request);

    const [, errorPath] = error.stack.toString().split(/\r\n|\n/);

    expect(response).to.deep.equal({
      code: GrpcErrorCodes.INTERNAL,
      info: cbor.encode({
        message: `${error.message} ${errorPath.trim()}`,
        metadata: {
          stack: error.stack,
        },
      }).toString('base64'),
    });
  });

  it('should respond with error if method throws DPPValidationError', async () => {
    const dppValidationError = new DPPValidationError('Some error', [new SomeConsensusError('Consensus error')]);

    methodMock.throws(dppValidationError);

    handler = wrapInErrorHandler(
      methodMock, { respondWithInternalError: true },
    );

    const response = await handler(request);

    expect(response).to.deep.equal({
      code: dppValidationError.getCode(),
      info: cbor.encode(dppValidationError.getInfo()).toString('base64'),
    });
  });
});
