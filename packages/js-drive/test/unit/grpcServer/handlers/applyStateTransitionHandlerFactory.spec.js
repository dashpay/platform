const { ApplyStateTransitionResponse, ApplyStateTransitionRequest } = require('@dashevo/drive-grpc');
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const InvalidStateTranistionError = require(
  '@dashevo/dpp/lib/stateTransition/errors/InvalidStateTransitionError',
);

const {
  server: {
    error: {
      InvalidArgumentGrpcError,
      FailedPreconditionGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const StateViewTransactionMock = require('../../../../lib/test/mock/StateViewTransactionMock');
const applyStateTransitionHandlerFactory = require('../../../../lib/grpcServer/handlers/applyStateTransitionHandlerFactory');
const GrpcCallMock = require('../../../../lib/test/mock/GrpcCallMock');
const getStateTransitionsFixture = require('../../../../lib/test/fixtures/getStateTransitionsFixture');

const BlockExecutionState = require('../../../../lib/updateState/BlockExecutionState');

describe('applyStateTransitionHandlerFactory', () => {
  let applyStateTransitionHandler;
  let call;
  let stateViewTransactionMock;
  let request;
  let dppMock;
  let applyStateTransitionMock;
  let blockExecutionState;
  let stateTransition;

  beforeEach(function beforeEach() {
    ([stateTransition] = getStateTransitionsFixture(1));

    blockExecutionState = new BlockExecutionState();

    stateViewTransactionMock = new StateViewTransactionMock(this.sinon);
    stateViewTransactionMock.isTransactionStarted = true;

    dppMock = createDPPMock(this.sinon);

    dppMock.stateTransition.createFromSerialized.resolves(stateTransition);

    applyStateTransitionMock = this.sinon.stub();

    applyStateTransitionHandler = applyStateTransitionHandlerFactory(
      stateViewTransactionMock,
      dppMock,
      applyStateTransitionMock,
      blockExecutionState,
    );

    request = new ApplyStateTransitionRequest();
    request.getBlockHeight = this.sinon.stub().returns(1);
    request.getBlockHash = this.sinon.stub().returns('hash');
    request.getStateTransition = this.sinon.stub().returns(
      stateTransition.serialize(),
    );

    call = new GrpcCallMock(this.sinon, request);
  });

  it('should throw FailedPreconditionGrpcError if transaction is not started', async () => {
    stateViewTransactionMock.isTransactionStarted = false;

    try {
      await applyStateTransitionHandler(call);
      expect.fail('should throw an FailedPreconditionGrpcError error');
    } catch (error) {
      expect(error).to.be.an.instanceOf(FailedPreconditionGrpcError);
      expect(error.getMessage()).to.equal('Failed precondition: Transaction is not started');
      expect(applyStateTransitionMock).to.not.be.called();
    }
  });

  it('should throw InvalidArgumentGrpcError if blockHeight param is missed', async () => {
    request.getBlockHeight.returns(undefined);

    try {
      await applyStateTransitionHandler(call);
      expect.fail('should throw an InvalidArgumentGrpcError error');
    } catch (error) {
      expect(error).to.be.an.instanceOf(InvalidArgumentGrpcError);
      expect(error.message).to.be.equal('Invalid argument: Block height is not specified');
      expect(applyStateTransitionMock).to.not.be.called();
    }
  });

  it('should throw InvalidArgumentGrpcError if blockHash param is missed', async () => {
    request.getBlockHash.returns(undefined);

    try {
      await applyStateTransitionHandler(call);
      expect.fail('should throw an InvalidArgumentGrpcError error');
    } catch (error) {
      expect(error).to.be.an.instanceOf(InvalidArgumentGrpcError);
      expect(error.message).to.be.equal('Invalid argument: Block hash is not specified');
      expect(applyStateTransitionMock).to.not.be.called();
    }
  });

  it('should throw InvalidArgumentGrpcError if state transition is invalid', async () => {
    dppMock.stateTransition.createFromSerialized.throws(
      new InvalidStateTranistionError([], stateTransition.toJSON()),
    );

    try {
      await applyStateTransitionHandler(call);

      expect.fail('should throw an InvalidArgumentGrpcError error');
    } catch (error) {
      expect(error).to.be.an.instanceOf(InvalidArgumentGrpcError);
      expect(error.message).to.be.equal('Invalid argument: Invalid State Transition');
      expect(applyStateTransitionMock).to.not.be.called();
    }
  });

  it('should apply state transition', async () => {
    applyStateTransitionMock.resolves({ svContract: {} });

    const response = await applyStateTransitionHandler(call);

    expect(applyStateTransitionMock).to.have.been.calledOnceWith(
      stateTransition,
      Buffer.from('hash').toString('hex'),
      1,
      stateViewTransactionMock,
    );

    expect(response).to.be.an.instanceOf(ApplyStateTransitionResponse);
    expect(blockExecutionState.getContracts()).to.have.lengthOf(1);
  });
});
