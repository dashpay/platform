const { StartTransactionResponse } = require('@dashevo/drive-grpc');

const {
  server: {
    error: {
      FailedPreconditionGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const startTransactionHandlerFactory = require('../../../../lib/grpcServer/handlers/startTransactionHandlerFactory');
const StateViewTransactionMock = require('../../../../lib/test/mock/StateViewTransactionMock');
const GrpcCallMock = require('../../../../lib/test/mock/GrpcCallMock');

describe('startTransactionHandlerFactory', () => {
  let startTransactionHandler;
  let call;
  let stateViewTransactionMock;

  beforeEach(function beforeEach() {
    stateViewTransactionMock = new StateViewTransactionMock(this.sinon);
    startTransactionHandler = startTransactionHandlerFactory(stateViewTransactionMock);
    call = new GrpcCallMock(this.sinon, {});
  });

  it('should throw an error if transaction is already started', async () => {
    stateViewTransactionMock.isTransactionStarted = true;

    try {
      await startTransactionHandler(call);

      expect.fail('should throw an error');
    } catch (error) {
      expect(error).to.be.an.instanceOf(FailedPreconditionGrpcError);
      expect(error.getMessage()).to.equal('Failed precondition: Transaction is already started');
    }
  });

  it('should start state view transaction', async () => {
    const response = await startTransactionHandler(call);

    expect(stateViewTransactionMock.start).to.be.calledOnce();

    expect(response).to.be.an.instanceOf(StartTransactionResponse);
  });
});
