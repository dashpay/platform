const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  ApplyStateTransitionResponse,
} = require('@dashevo/dapi-grpc');

const DashPlatformProtocol = require('@dashevo/dpp');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const applyStateTransitionHandlerFactory = require(
  '../../../../../lib/grpcServer/handlers/platform/applyStateTransitionHandlerFactory',
);

describe('applyStateTransitionHandlerFactory', () => {
  let call;
  let rpcClientMock;
  let applyStateTransitionHandler;
  let response;
  let stateTransitionFixture;
  let log;
  let code;
  let handleResponseMock;

  beforeEach(async function beforeEach() {
    const dpp = new DashPlatformProtocol();

    const dataContractFixture = getDataContractFixture();
    stateTransitionFixture = dpp.dataContract.createStateTransition(dataContractFixture);

    call = new GrpcCallMock(this.sinon, {
      getStateTransition: this.sinon.stub().returns(stateTransitionFixture.serialize()),
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

    handleResponseMock = this.sinon.stub();

    applyStateTransitionHandler = applyStateTransitionHandlerFactory(
      rpcClientMock,
      handleResponseMock,
    );
  });

  afterEach(function afterEach() {
    this.sinon.restore();
  });

  it('should throw an InvalidArgumentGrpcError if stateTransition is not specified', async () => {
    call.request.getStateTransition.returns(null);

    try {
      await applyStateTransitionHandler(call);

      expect.fail('InvalidArgumentGrpcError was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('State Transition is not specified');
      expect(rpcClientMock.request).to.not.be.called();
      expect(handleResponseMock).to.not.be.called();
    }
  });

  it('should return valid result', async () => {
    const result = await applyStateTransitionHandler(call);

    const tx = stateTransitionFixture.serialize().toString('base64');

    expect(result).to.be.an.instanceOf(ApplyStateTransitionResponse);
    expect(rpcClientMock.request).to.be.calledOnceWith('broadcast_tx_commit', { tx });
    expect(handleResponseMock).to.be.calledTwice();
    expect(handleResponseMock).to.be.calledWith({ code, log });
  });

  it('should throw error if handleResponseMock throws an error', async () => {
    const error = new Error();

    handleResponseMock.throws(error);

    try {
      await applyStateTransitionHandler(call);

      expect.fail('InternalGrpcError was not thrown');
    } catch (e) {
      expect(e).to.equal(error);
    }
  });

  it('should throw an error when handleResponse throws an error', async () => {
    const error = new Error();
    handleResponseMock.throws(error);

    try {
      await applyStateTransitionHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.equal(error);
    }
  });
});
