const {
  tendermint: {
    abci: {
      ResponseDeliverTx,
    },
  },
} = require('@dashevo/abci/types');

const DashPlatformProtocol = require('@dashevo/dpp');

const ValidationResult = require('@dashevo/dpp/lib/validation/ValidationResult');

const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const SomeConsensusError = require('@dashevo/dpp/lib/test/mocks/SomeConsensusError');
const BlockExecutionContextMock = require('../../../../lib/test/mock/BlockExecutionContextMock');

const deliverTxHandlerFactory = require('../../../../lib/abci/handlers/deliverTxHandlerFactory');

const LoggerMock = require('../../../../lib/test/mock/LoggerMock');
const DPPValidationAbciError = require('../../../../lib/abci/errors/DPPValidationAbciError');

const InvalidArgumentAbciError = require('../../../../lib/abci/errors/InvalidArgumentAbciError');

describe('deliverTxHandlerFactory', () => {
  let deliverTxHandler;
  let dataContractRequest;
  let documentRequest;
  let identity;
  let dppMock;
  let stateRepositoryMock;
  let documentsBatchTransitionFixture;
  let dataContractCreateTransitionFixture;
  let dpp;
  let unserializeStateTransitionMock;
  let blockExecutionContextMock;
  let validationResult;

  beforeEach(async function beforeEach() {
    const dataContractFixture = getDataContractFixture();
    const documentFixture = getDocumentFixture();

    dpp = new DashPlatformProtocol();
    await dpp.initialize();

    documentsBatchTransitionFixture = dpp.document.createStateTransition({
      create: documentFixture,
    });

    dataContractCreateTransitionFixture = dpp
      .dataContract.createStateTransition(dataContractFixture);

    documentRequest = {
      tx: documentsBatchTransitionFixture.toBuffer(),
    };

    dataContractRequest = {
      tx: dataContractCreateTransitionFixture.toBuffer(),
    };

    dppMock = createDPPMock(this.sinon);

    validationResult = new ValidationResult();

    dppMock
      .stateTransition
      .validateState
      .resolves(validationResult);

    stateRepositoryMock = createStateRepositoryMock(this.sinon);

    identity = getIdentityFixture();

    stateRepositoryMock.fetchIdentity.resolves(identity);

    dppMock.getStateRepository.returns(stateRepositoryMock);

    unserializeStateTransitionMock = this.sinon.stub();

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    blockExecutionContextMock.getHeader.returns({
      height: 42,
    });

    const loggerMock = new LoggerMock(this.sinon);

    deliverTxHandler = deliverTxHandlerFactory(
      unserializeStateTransitionMock,
      dppMock,
      blockExecutionContextMock,
      loggerMock,
    );
  });

  it('should apply a DocumentsBatchTransition and return ResponseDeliverTx', async () => {
    unserializeStateTransitionMock.resolves(documentsBatchTransitionFixture);

    const response = await deliverTxHandler(documentRequest);

    expect(response).to.be.an.instanceOf(ResponseDeliverTx);
    expect(response.code).to.equal(0);

    expect(unserializeStateTransitionMock).to.be.calledOnceWith(
      documentsBatchTransitionFixture.toBuffer(),
    );
    expect(dppMock.stateTransition.validateState).to.be.calledOnceWith(
      documentsBatchTransitionFixture,
    );
    expect(dppMock.stateTransition.apply).to.be.calledOnceWith(
      documentsBatchTransitionFixture,
    );
    expect(blockExecutionContextMock.addDataContract).to.not.be.called();

    const stateTransitionFee = documentsBatchTransitionFixture.calculateFee();

    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWith(
      documentsBatchTransitionFixture.getOwnerId(),
    );

    identity.reduceBalance(stateTransitionFee);

    expect(stateRepositoryMock.storeIdentity).to.be.calledOnceWith(identity);

    expect(blockExecutionContextMock.incrementCumulativeFees).to.be.calledOnceWith(
      stateTransitionFee,
    );
  });

  it('should apply a DataContractCreateTransition, add it to block execution state and return ResponseDeliverTx', async () => {
    unserializeStateTransitionMock.resolves(dataContractCreateTransitionFixture);

    const response = await deliverTxHandler(dataContractRequest);

    expect(response).to.be.an.instanceOf(ResponseDeliverTx);
    expect(response.code).to.equal(0);

    expect(unserializeStateTransitionMock).to.be.calledOnceWith(
      dataContractCreateTransitionFixture.toBuffer(),
    );
    expect(dppMock.stateTransition.validateState).to.be.calledOnceWith(
      dataContractCreateTransitionFixture,
    );
    expect(dppMock.stateTransition.apply).to.be.calledOnceWith(
      dataContractCreateTransitionFixture,
    );
    expect(blockExecutionContextMock.addDataContract).to.be.calledOnceWith(
      dataContractCreateTransitionFixture.getDataContract(),
    );

    expect(blockExecutionContextMock.incrementCumulativeFees).to.be.calledOnceWith(
      dataContractCreateTransitionFixture.calculateFee(),
    );
  });

  it('should throw DPPValidationAbciError if a state transition is invalid against state', async () => {
    unserializeStateTransitionMock.resolves(dataContractCreateTransitionFixture);

    const error = new SomeConsensusError('Consensus error');

    validationResult.addError(error);

    try {
      await deliverTxHandler(documentRequest);

      expect.fail('should throw InvalidArgumentAbciError error');
    } catch (e) {
      expect(e).to.be.instanceOf(DPPValidationAbciError);
      expect(e.getCode()).to.equal(error.getCode());
      expect(e.getData()).to.deep.equal({
        arguments: ['Consensus error'],
      });
      expect(blockExecutionContextMock.incrementCumulativeFees).to.not.be.called();
    }
  });

  it('should throw DPPValidationAbciError if a state transition is not valid', async () => {
    const errorMessage = 'Invalid structure';
    const error = new InvalidArgumentAbciError(errorMessage);

    unserializeStateTransitionMock.throws(error);

    try {
      await deliverTxHandler(documentRequest);

      expect.fail('should throw InvalidArgumentAbciError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentAbciError);
      expect(e.getMessage()).to.equal(errorMessage);
      expect(e.getCode()).to.equal(GrpcErrorCodes.INVALID_ARGUMENT);
      expect(blockExecutionContextMock.incrementCumulativeFees).to.not.be.called();
      expect(dppMock.stateTransition.validate).to.not.be.called();
    }
  });
});
