const cbor = require('cbor');

const wrapInErrorHandlerFactory = require('../../../../lib/server/error/wrapInErrorHandlerFactory');
const InternalGrpcError = require('../../../../lib/server/error/InternalGrpcError');
const VerboseInternalGrpcError = require('../../../../lib/server/error/VerboseInternalGrpcError');
const InvalidArgumentGrpcError = require('../../../../lib/server/error/InvalidArgumentGrpcError');

describe('wrapInErrorHandlerFactory', () => {
  let loggerMock;
  let wrapInErrorHandler;
  let rpcMethod;
  let callback;
  let call;

  beforeEach(function beforeEach() {
    loggerMock = {
      error: this.sinon.stub(),
    };

    wrapInErrorHandler = wrapInErrorHandlerFactory(loggerMock, false);

    rpcMethod = this.sinon.stub();
    callback = this.sinon.stub();
    call = {};
  });

  it('should return wrapped RPC method', () => {
    const wrappedRpcMethod = wrapInErrorHandler(rpcMethod);

    expect(wrappedRpcMethod).to.be.a('function');
    expect(rpcMethod).to.not.be.called();
  });

  describe('wrapped RPC method', () => {
    it('should call a method', async () => {
      const result = 42;

      rpcMethod.resolves(result);

      const wrappedRpcMethod = wrapInErrorHandler(rpcMethod);

      await wrappedRpcMethod(call, callback);

      expect(rpcMethod).to.be.calledOnceWith(call);
      expect(callback).to.be.calledOnceWith(null, result);
      expect(loggerMock.error).to.not.be.called();
    });

    it('should call callback with GrpcError if it was thrown from the method', async () => {
      const wrappedRpcMethod = wrapInErrorHandler(rpcMethod);

      const grpcError = new InvalidArgumentGrpcError('Something wrong');

      rpcMethod.throws(grpcError);

      await wrappedRpcMethod(call, callback);

      expect(rpcMethod).to.be.calledOnceWith(call);
      expect(callback).to.be.calledOnceWith(grpcError, null);
      expect(loggerMock.error).to.not.be.called();
    });

    it('should log and call callback with InternalGrpcError if some error except GrpcError was thrown from the method', async () => {
      const wrappedRpcMethod = wrapInErrorHandler(rpcMethod);

      const someError = new Error();

      rpcMethod.throws(someError);

      await wrappedRpcMethod(call, callback);

      expect(rpcMethod).to.be.calledOnceWith(call);

      expect(callback).to.be.calledOnce();
      expect(callback.getCall(0).args).to.have.lengthOf(2);

      const [grpcError] = callback.getCall(0).args;

      expect(grpcError).to.be.instanceOf(InternalGrpcError);
      expect(grpcError.getError()).to.equal(someError);

      expect(loggerMock.error).to.be.calledOnceWith(someError);
    });

    it('should return VerboseInternalGrpcError in development environment', async () => {
      wrapInErrorHandler = wrapInErrorHandlerFactory(loggerMock, false);

      const wrappedRpcMethod = wrapInErrorHandler(rpcMethod);

      const someError = new Error('error');

      const [, errorPath] = someError.stack.toString().split(/\r\n|\n/);

      const errorMessage = `${someError.message} ${errorPath.trim()}`;

      rpcMethod.throws(someError);

      await wrappedRpcMethod(call, callback);

      expect(rpcMethod).to.be.calledOnceWith(call);

      expect(rpcMethod).to.be.calledOnceWith(call);

      expect(callback).to.be.calledOnce();
      expect(callback.getCall(0).args).to.have.lengthOf(2);

      const [grpcError] = callback.getCall(0).args;

      expect(grpcError).to.be.instanceOf(VerboseInternalGrpcError);
      expect(grpcError.getError()).to.equal(someError);
      expect(grpcError.getMessage()).to.equal(errorMessage);
      expect(grpcError.getRawMetadata()).to.deep.equal({
        'stack-bin': cbor.encode(someError.stack),
      });

      expect(loggerMock.error).to.be.calledOnceWith(someError);
    });
  });
});
