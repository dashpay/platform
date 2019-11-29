const {
  server: {
    error: {
      InternalGrpcError,
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  UpdateStateTransitionResponse,
} = require('@dashevo/dapi-grpc');

const DashPlatformProtocol = require('@dashevo/dpp');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const updateStateHandlerFactory = require(
  '../../../../../lib/grpcServer/handlers/core/updateStateHandlerFactory',
);

describe('updateStateHandlerFactory', () => {
  let call;
  let rpcClientMock;
  let updateStateHandler;
  let response;
  let stateTransitionFixture;
  let log;

  beforeEach(async function beforeEach() {
    const dpp = new DashPlatformProtocol();

    const dataContractFixture = getDataContractFixture();
    stateTransitionFixture = dpp.dataContract.createStateTransition(dataContractFixture);

    call = new GrpcCallMock(this.sinon, {
      getData: this.sinon.stub().returns(stateTransitionFixture.serialize()),
    });

    log = JSON.stringify({
      error: {
        message: 'some message',
        data: {
          error: 'some data',
        },
      },
    });

    const code = 0;

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

    updateStateHandler = updateStateHandlerFactory(
      rpcClientMock,
    );
  });

  afterEach(function afterEach() {
    this.sinon.restore();
  });

  it('should throw an InvalidArgumentGrpcError if stateTransition is not specified', async () => {
    call.request.getData.returns(null);

    try {
      await updateStateHandler(call);

      expect.fail('InvalidArgumentGrpcError was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('Invalid argument: State Transition is not specified');
      expect(rpcClientMock.request).to.not.be.called();
    }
  });

  it('should return valid result', async () => {
    const result = await updateStateHandler(call);

    const tx = stateTransitionFixture.serialize().toString('base64');

    expect(result).to.be.an.instanceOf(UpdateStateTransitionResponse);
    expect(rpcClientMock.request).to.be.calledOnceWith('broadcast_tx_commit', { tx });
  });

  it('should throw InternalGrpcError if Tendermint Core returns check_tx with code 1 (internal error)', async () => {
    response.result.check_tx.code = 1;

    const tx = stateTransitionFixture.serialize().toString('base64');

    try {
      await updateStateHandler(call);

      expect.fail('InternalGrpcError was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InternalGrpcError);
      expect(e.getMetadata()).to.deep.equal({ error: 'some data' });
      expect(rpcClientMock.request).to.be.calledOnceWith('broadcast_tx_commit', { tx });
    }
  });

  it('should throw InvalidArgumentGrpcError if Tendermint Core returns check_tx with code 2 (invalid argument)', async () => {
    response.result.check_tx.code = 2;

    const tx = stateTransitionFixture.serialize().toString('base64');

    try {
      await updateStateHandler(call);

      expect.fail('InvalidArgumentGrpcError was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('Invalid argument: some message');
      expect(e.getMetadata()).to.deep.equal({ error: 'some data' });
      expect(rpcClientMock.request).to.be.calledOnceWith('broadcast_tx_commit', { tx });
    }
  });

  it('should throw InternalGrpcError if Tendermint Core returns check_tx with unknown code', async () => {
    response.result.check_tx.code = 4;

    const tx = stateTransitionFixture.serialize().toString('base64');

    try {
      await updateStateHandler(call);

      expect.fail('InternalGrpcError was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InternalGrpcError);
      expect(e.getMetadata()).to.deep.equal({ error: 'some data' });
      expect(rpcClientMock.request).to.be.calledOnceWith('broadcast_tx_commit', { tx });
    }
  });

  it('should throw InternalGrpcError if Tendermint Core returns deliver_tx with code 1 (internal error)', async () => {
    response.result.deliver_tx.code = 1;

    const tx = stateTransitionFixture.serialize().toString('base64');

    try {
      await updateStateHandler(call);

      expect.fail('InternalGrpcError was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InternalGrpcError);
      expect(e.getMetadata()).to.deep.equal({ error: 'some data' });
      expect(rpcClientMock.request).to.be.calledOnceWith('broadcast_tx_commit', { tx });
    }
  });

  it('should throw InvalidArgumentGrpcError if Tendermint Core returns deliver_tx with code 2 (invalid argument)', async () => {
    response.result.deliver_tx.code = 2;

    const tx = stateTransitionFixture.serialize().toString('base64');

    try {
      await updateStateHandler(call);

      expect.fail('InvalidArgumentGrpcError was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('Invalid argument: some message');
      expect(e.getMetadata()).to.deep.equal({ error: 'some data' });
      expect(rpcClientMock.request).to.be.calledOnceWith('broadcast_tx_commit', { tx });
    }
  });

  it('should throw InternalGrpcError if Tendermint Core returns deliver_tx with unknown code', async () => {
    response.result.deliver_tx.code = 4;

    const tx = stateTransitionFixture.serialize().toString('base64');

    try {
      await updateStateHandler(call);

      expect.fail('InternalGrpcError was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InternalGrpcError);
      expect(e.getMetadata()).to.deep.equal({ error: 'some data' });
      expect(rpcClientMock.request).to.be.calledOnceWith('broadcast_tx_commit', { tx });
    }
  });

  it('should return error if timeout happened');

  it('should return InternalGrpcError if Tendermint Core throws an error', async () => {
    const error = {
      code: -32603,
      message: 'Internal error',
      data: 'just error',
    };

    rpcClientMock.request.resolves({
      id: '',
      jsonrpc: '2.0',
      error,
    });

    try {
      await updateStateHandler(call);

      expect.fail('InternalGrpcError was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InternalGrpcError);
      expect(e.getError()).to.deep.equal(error);
    }
  });
});
