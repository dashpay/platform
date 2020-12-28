const {
  server: {
    error: {
      InvalidArgumentGrpcError,
      FailedPreconditionGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    BroadcastStateTransitionResponse,
  },
} = require('@dashevo/dapi-grpc');

const DashPlatformProtocol = require('@dashevo/dpp');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const broadcastStateTransitionHandlerFactory = require(
  '../../../../../lib/grpcServer/handlers/platform/broadcastStateTransitionHandlerFactory',
);

describe('broadcastStateTransitionHandlerFactory', () => {
  let call;
  let rpcClientMock;
  let broadcastStateTransitionHandler;
  let response;
  let stateTransitionFixture;
  let log;
  let code;
  let handleAbciResponseErrorMock;

  beforeEach(async function beforeEach() {
    const dpp = new DashPlatformProtocol();

    const dataContractFixture = getDataContractFixture();
    stateTransitionFixture = dpp.dataContract.createStateTransition(dataContractFixture);

    call = new GrpcCallMock(this.sinon, {
      getStateTransition: this.sinon.stub().returns(stateTransitionFixture.toBuffer()),
    });

    log = JSON.stringify({
      error: {
        message: 'some message',
        data: {
          error: 'some data',
        },
      },
    });

    code = 0;

    response = {
      id: '',
      jsonrpc: '2.0',
      error: '',
      result: {
        check_tx: { code, log },
        deliver_tx: { code, log },
        hash:
        'B762539A7C17C33A65C46727BFCF2C701390E6AD7DE5190B6CC1CF843CA7E262',
        height: '24',
      },
    };

    rpcClientMock = {
      request: this.sinon.stub().resolves(response),
    };

    handleAbciResponseErrorMock = this.sinon.stub();

    broadcastStateTransitionHandler = broadcastStateTransitionHandlerFactory(
      rpcClientMock,
      handleAbciResponseErrorMock,
    );
  });

  afterEach(function afterEach() {
    this.sinon.restore();
  });

  it('should throw an InvalidArgumentGrpcError if stateTransition is not specified', async () => {
    call.request.getStateTransition.returns(null);

    try {
      await broadcastStateTransitionHandler(call);

      expect.fail('InvalidArgumentGrpcError was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('State Transition is not specified');
      expect(rpcClientMock.request).to.not.be.called();
      expect(handleAbciResponseErrorMock).to.not.be.called();
    }
  });

  it('should return valid result', async () => {
    const result = await broadcastStateTransitionHandler(call);

    const tx = stateTransitionFixture.toBuffer().toString('base64');

    expect(result).to.be.an.instanceOf(BroadcastStateTransitionResponse);
    expect(rpcClientMock.request).to.be.calledOnceWith('broadcast_tx_commit', { tx });
    expect(handleAbciResponseErrorMock).to.not.be.called();
  });

  it('should throw error if checkTx.code !== 0', async () => {
    const error = new InvalidArgumentGrpcError('Some error');
    code = 2;

    response.result.check_tx.code = code;

    handleAbciResponseErrorMock.throws(error);

    rpcClientMock.request.resolves(response);

    try {
      await broadcastStateTransitionHandler(call);

      expect.fail('InternalGrpcError was not thrown');
    } catch (e) {
      expect(e).to.be.equal(error);
    }
  });
  it('should throw error if deliverTx.code !== 0', async () => {
    const error = new InvalidArgumentGrpcError('Some error');
    code = 2;

    response.result.deliver_tx.code = code;

    handleAbciResponseErrorMock.throws(error);

    rpcClientMock.request.resolves(response);

    try {
      await broadcastStateTransitionHandler(call);

      expect.fail('InternalGrpcError was not thrown');
    } catch (e) {
      expect(e).to.be.equal(error);
    }
  });

  it('should throw an error if transaction broadcast returns error', async () => {
    const error = { code: -1, message: "Something didn't work", data: 'Some data' };

    response.error = error;

    try {
      await broadcastStateTransitionHandler(call);
    } catch (e) {
      expect(e.message).to.equal(error.message);
      expect(e.data).to.equal(error.data);
      expect(e.code).to.equal(error.code);
    }
  });

  it('should throw FailedPreconditionGrpcError if transaction was broadcasted twice', async () => {
    const error = {
      code: -32603,
      message: 'Internal error',
      data: 'error on broadcastTxCommit: tx already exists in cache',
    };

    response.error = error;

    try {
      await broadcastStateTransitionHandler(call);
    } catch (e) {
      expect(e).to.be.an.instanceOf(FailedPreconditionGrpcError);
      expect(e.getMessage()).to.equal(`Failed precondition: ${error.data}`);
    }
  });
});
