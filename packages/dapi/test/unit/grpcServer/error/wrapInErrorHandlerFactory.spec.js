const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');

const wrapInErrorHandlerFactory = require('../../../../lib/grpcServer/error/wrapInErrorHandlerFactory');
const InternalError = require('../../../../lib/grpcServer/error/InternalError');
const InvalidArgumentError = require('../../../../lib/grpcServer/error/InvalidArgumentError');

use(sinonChai);
use(chaiAsPromised);
use(dirtyChai);

describe('wrapInErrorHandlerFactory', () => {
  beforeEach(function beforeEach() {
    if (!this.sinon) {
      this.sinon = sinon.createSandbox();
    } else {
      this.sinon.restore();
    }
  });

  afterEach(function afterEach() {
    this.sinon.restore();
  });

  let loggerMock;
  let wrapInErrorHandler;
  let rpcMethod;
  let callback;
  let call;

  beforeEach(function beforeEach() {
    loggerMock = {
      error: this.sinon.stub(),
    };

    wrapInErrorHandler = wrapInErrorHandlerFactory(loggerMock);

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
    it('should call a method', () => {
      const wrappedRpcMethod = wrapInErrorHandler(rpcMethod);

      wrappedRpcMethod(call, callback);

      expect(rpcMethod).to.be.calledOnceWith(call, callback);
      expect(callback).to.not.be.called();
      expect(loggerMock.error).to.not.be.called();
    });

    it('should call callback with GrpcError if it was thrown from the method', () => {
      const wrappedRpcMethod = wrapInErrorHandler(rpcMethod);

      const grpcError = new InvalidArgumentError('Something wrong');

      rpcMethod.throws(grpcError);

      wrappedRpcMethod(call, callback);

      expect(rpcMethod).to.be.calledOnceWith(call, callback);
      expect(callback).to.be.calledOnceWith(grpcError);
      expect(loggerMock.error).to.not.be.called();
    });

    it('should log and call callback with InternalError if some error except GrpcError was thrown from the method', () => {
      const wrappedRpcMethod = wrapInErrorHandler(rpcMethod);

      const someError = new Error();

      rpcMethod.throws(someError);

      wrappedRpcMethod(call, callback);

      expect(rpcMethod).to.be.calledOnceWith(call, callback);

      expect(callback).to.be.calledOnce();
      expect(callback.getCall(0).args).to.have.lengthOf(2);

      const [grpcError] = callback.getCall(0).args;

      expect(grpcError).to.be.instanceOf(InternalError);
      expect(grpcError.getError()).to.equal(someError);

      expect(loggerMock.error).to.be.calledOnceWith(someError);
    });
  });
});
