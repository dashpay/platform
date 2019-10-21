const { ApplyStateTransitionResponse, ApplyStateTransitionRequest } = require('@dashevo/drive-grpc');
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const InvalidSTPacketError = require('@dashevo/dpp/lib/stPacket/errors/InvalidSTPacketError');
const ConsensusError = require('@dashevo/dpp/lib/errors/ConsensusError');
const ValidationResult = require('@dashevo/dpp/lib/validation/ValidationResult');

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
const getSTPacketsFixture = require('../../../../lib/test/fixtures/getSTPacketsFixture');
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
  let stPacket;
  let stHeader;

  beforeEach(function beforeEach() {
    ([stPacket] = getSTPacketsFixture());
    ([stHeader] = getStateTransitionsFixture(1));

    blockExecutionState = new BlockExecutionState();

    stateViewTransactionMock = new StateViewTransactionMock(this.sinon);
    stateViewTransactionMock.isTransactionStarted = true;

    dppMock = createDPPMock(this.sinon);

    dppMock.packet.createFromSerialized.resolves(stPacket);
    dppMock.packet.verify.resolves(new ValidationResult());

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
    request.getStateTransitionPacket = this.sinon.stub().returns(
      stPacket.serialize(),
    );
    request.getStateTransitionHeader = this.sinon.stub().returns(
      Buffer.from(stHeader.serialize(), 'hex'),
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

  it('should throw InvalidArgumentGrpcError if stHeaderPacket param is missed', async () => {
    request.getStateTransitionPacket.returns(undefined);

    try {
      await applyStateTransitionHandler(call);
      expect.fail('should throw an InvalidArgumentGrpcError error');
    } catch (error) {
      expect(error).to.be.an.instanceOf(InvalidArgumentGrpcError);
      expect(error.message).to.be.equal('Invalid argument: stateTransitionPacket is not specified');
      expect(applyStateTransitionMock).to.not.be.called();
    }
  });

  it('should thrown InvalidArgumentGrpcError if stHeaderHeader param is missed', async () => {
    request.getStateTransitionHeader.returns(undefined);

    try {
      await applyStateTransitionHandler(call);
      expect.fail('should throw an InvalidArgumentGrpcError error');
    } catch (error) {
      expect(error).to.be.an.instanceOf(InvalidArgumentGrpcError);
      expect(error.message).to.be.equal('Invalid argument: stateTransitionHeader is not specified');
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
      expect(error.message).to.be.equal('Invalid argument: blockHeight is not specified');
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
      expect(error.message).to.be.equal('Invalid argument: blockHash is not specified');
      expect(applyStateTransitionMock).to.not.be.called();
    }
  });

  it('should throw InvalidArgumentGrpcError if ST Packet is invalid', async () => {
    dppMock.packet.createFromSerialized.throws(
      new InvalidSTPacketError([], stPacket.toJSON()),
    );

    try {
      await applyStateTransitionHandler(call);

      expect.fail('should throw an InvalidArgumentGrpcError error');
    } catch (error) {
      expect(error).to.be.an.instanceOf(InvalidArgumentGrpcError);
      expect(error.message).to.be.equal('Invalid argument: Invalid ST Packet');
      expect(applyStateTransitionMock).to.not.be.called();
    }
  });

  it('should InvalidArgumentGrpcError if ST Header is invalid', async () => {
    const stHeaderHeader = Buffer.from(
      'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a0',
      'hex',
    );

    request.getStateTransitionHeader.returns(stHeaderHeader);

    try {
      await applyStateTransitionHandler(call);

      expect.fail('should throw an InvalidArgumentGrpcError error');
    } catch (error) {
      expect(error).to.be.an.instanceOf(InvalidArgumentGrpcError);
      expect(error.message).to.equal('Invalid argument: Invalid "stateTransitionHeader": The value of "offset" is out of range. It must be >= 0 and <= 23. Received 37');
      expect(applyStateTransitionMock).to.not.be.called();
    }
  });

  it('should InvalidArgumentGrpcError if ST Packet and/or ST Header are invalid', async () => {
    dppMock.packet.verify.resolves(
      new ValidationResult([new ConsensusError('error')]),
    );

    try {
      await applyStateTransitionHandler(call);

      expect.fail('should throw an InvalidArgumentGrpcError error');
    } catch (error) {
      expect(error).to.be.an.instanceOf(InvalidArgumentGrpcError);
      expect(error.message).to.be.equal('Invalid argument: Invalid "stPacket" and "stHeader"');
      expect(applyStateTransitionMock).to.not.be.called();
    }
  });

  it('should apply state transition', async () => {
    applyStateTransitionMock.resolves({ svContract: {} });

    const response = await applyStateTransitionHandler(call);

    expect(applyStateTransitionMock).to.be.calledOnce();

    expect(applyStateTransitionMock.getCall(0).args[0]).to.equal(stPacket);
    expect(applyStateTransitionMock.getCall(0).args[1]).to.respondTo('serialize');
    expect(applyStateTransitionMock.getCall(0).args[1].serialize()).to.equal(stHeader.serialize());
    expect(applyStateTransitionMock.getCall(0).args[2]).to.equal(
      Buffer.from('hash').toString('hex'),
    );
    expect(applyStateTransitionMock.getCall(0).args[3]).to.equal(1);
    expect(applyStateTransitionMock.getCall(0).args[4]).to.equal(stateViewTransactionMock);

    expect(response).to.be.an.instanceOf(ApplyStateTransitionResponse);
    expect(blockExecutionState.getContracts()).to.have.lengthOf(1);
  });
});
