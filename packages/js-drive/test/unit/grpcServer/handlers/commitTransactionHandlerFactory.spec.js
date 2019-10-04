const { CommitTransactionResponse, CommitTransactionRequest } = require('@dashevo/drive-grpc');

const commitTransactionHandlerFactory = require('../../../../lib/grpcServer/handlers/commitTransactionHandlerFactory');
const GrpcCallMock = require('../../../../lib/test/mock/GrpcCallMock');
const InternalGrpcError = require('../../../../lib/grpcServer/error/InternalGrpcError');
const BlockExecutionState = require('../../../../lib/updateState/BlockExecutionState');
const StateViewTransactionMock = require('../../../../lib/test/mock/StateViewTransactionMock');
const FailedPreconditionGrpcError = require('../../../../lib/grpcServer/error/FailedPreconditionGrpcError');
const getSVContractFixture = require('../../../../lib/test/fixtures/getSVContractFixture');

describe('commitTransactionHandlerFactory', () => {
  let commitTransactionHandler;
  let request;
  let call;
  let stateViewTransactionMock;
  let createContractDatabaseMock;
  let removeContractDatabaseMock;
  let blockExecutionState;
  let firstContract;

  beforeEach(function beforeEach() {
    blockExecutionState = new BlockExecutionState();

    stateViewTransactionMock = new StateViewTransactionMock(this.sinon);

    firstContract = getSVContractFixture();
    blockExecutionState.addContract(firstContract);
    blockExecutionState.addContract(getSVContractFixture());
    blockExecutionState.addContract(getSVContractFixture());

    createContractDatabaseMock = this.sinon.stub();
    removeContractDatabaseMock = this.sinon.stub();

    commitTransactionHandler = commitTransactionHandlerFactory(
      stateViewTransactionMock,
      createContractDatabaseMock,
      removeContractDatabaseMock,
      blockExecutionState,
    );

    request = new CommitTransactionRequest();
    request.getBlockHash = this.sinon.stub().returns(
      Buffer.from('hash').toString('hex'),
    );

    call = new GrpcCallMock(this.sinon, request);

    stateViewTransactionMock.isTransactionStarted = true;
  });

  it('should throw FailedPreconditionGrpcError if transaction is not started', async () => {
    stateViewTransactionMock.isTransactionStarted = false;

    try {
      await commitTransactionHandler(call);

      expect.fail('should throw an FailedPreconditionGrpcError error');
    } catch (error) {
      expect(error).to.be.an.instanceOf(FailedPreconditionGrpcError);

      expect(stateViewTransactionMock.commit).to.not.be.called();
      expect(createContractDatabaseMock).to.not.be.called();
    }
  });

  it('should throw InternalGrpcError and revert data if could not create a contract database', async () => {
    const createDatabaseError = new Error();

    createContractDatabaseMock.onCall(1).throws(createDatabaseError);

    try {
      await commitTransactionHandler(call);

      expect.fail('should throw an InternalGrpcError error');
    } catch (error) {
      expect(error).to.be.an.instanceOf(InternalGrpcError);
      expect(error.getError()).to.equal(createDatabaseError);

      expect(stateViewTransactionMock.commit).to.be.not.called();
      expect(stateViewTransactionMock.abort).to.be.calledOnce();

      expect(removeContractDatabaseMock).to.be.calledOnceWith(firstContract);
    }
  });

  it('should throw InternalGrpcError and revert data if could not commit transaction', async () => {
    const commitTransactionError = new Error();

    stateViewTransactionMock.commit.throws(commitTransactionError);

    try {
      await commitTransactionHandler(call);

      expect.fail('should throw an InternalGrpcError error');
    } catch (error) {
      expect(error).to.be.an.instanceOf(InternalGrpcError);
      expect(error.getError()).to.equal(commitTransactionError);

      expect(createContractDatabaseMock).to.be.calledThrice();

      expect(stateViewTransactionMock.abort).to.be.calledOnce();

      expect(removeContractDatabaseMock).to.be.calledThrice();
    }
  });

  it('should commit and create contract dbs', async () => {
    const response = await commitTransactionHandler(call);

    expect(response).to.be.an.instanceOf(CommitTransactionResponse);
    expect(blockExecutionState.getContracts()).to.have.lengthOf(0);

    expect(createContractDatabaseMock).to.be.calledThrice();

    expect(stateViewTransactionMock.commit).to.be.calledOnce();

    expect(stateViewTransactionMock.abort).to.not.be.called();
    expect(removeContractDatabaseMock).to.not.be.called();
  });
});
