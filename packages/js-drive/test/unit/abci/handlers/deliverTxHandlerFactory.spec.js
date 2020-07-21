const {
  abci: {
    ResponseDeliverTx,
  },
} = require('abci/types');

const DashPlatformProtocol = require('@dashevo/dpp');

const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const BlockExecutionStateMock = require('../../../../lib/test/mock/BlockExecutionStateMock');
const ValidationResult = require('../../../../lib/document/query/ValidationResult');

const deliverTxHandlerFactory = require('../../../../lib/abci/handlers/deliverTxHandlerFactory');

const InvalidArgumentAbciError = require('../../../../lib/abci/errors/InvalidArgumentAbciError');
const AbciError = require('../../../../lib/abci/errors/AbciError');
const ValidationError = require('../../../../lib/document/query/errors/ValidationError');

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
  let blockExecutionStateMock;

  beforeEach(function beforeEach() {
    const dataContractFixture = getDataContractFixture();
    const documentFixture = getDocumentFixture();

    dpp = new DashPlatformProtocol();
    documentsBatchTransitionFixture = dpp.document.createStateTransition({
      create: documentFixture,
    });

    dataContractCreateTransitionFixture = dpp
      .dataContract.createStateTransition(dataContractFixture);

    documentRequest = {
      tx: documentsBatchTransitionFixture.serialize(),
    };

    dataContractRequest = {
      tx: dataContractCreateTransitionFixture.serialize(),
    };

    dppMock = createDPPMock(this.sinon);

    dppMock
      .stateTransition
      .validateData
      .resolves({
        isValid: this.sinon.stub().returns(true),
      });

    stateRepositoryMock = createStateRepositoryMock(this.sinon);

    identity = getIdentityFixture();

    stateRepositoryMock.fetchIdentity.resolves(identity);

    dppMock.getStateRepository.returns(stateRepositoryMock);

    unserializeStateTransitionMock = this.sinon.stub();

    blockExecutionStateMock = new BlockExecutionStateMock(this.sinon);
    blockExecutionStateMock.getHeader.returns({
      height: 42,
    });

    const loggerMock = {
      debug: this.sinon.stub(),
      info: this.sinon.stub(),
    };

    deliverTxHandler = deliverTxHandlerFactory(
      unserializeStateTransitionMock,
      dppMock,
      blockExecutionStateMock,
      loggerMock,
    );
  });

  it('should apply a DocumentsBatchTransition and return ResponseDeliverTx', async () => {
    unserializeStateTransitionMock.resolves(documentsBatchTransitionFixture);

    const response = await deliverTxHandler(documentRequest);

    expect(response).to.be.an.instanceOf(ResponseDeliverTx);
    expect(response.code).to.equal(0);

    expect(unserializeStateTransitionMock).to.be.calledOnceWith(
      documentsBatchTransitionFixture.serialize(),
    );
    expect(dppMock.stateTransition.validateData).to.be.calledOnceWith(
      documentsBatchTransitionFixture,
    );
    expect(dppMock.stateTransition.apply).to.be.calledOnceWith(
      documentsBatchTransitionFixture,
    );
    expect(blockExecutionStateMock.addDataContract).to.not.be.called();

    const stateTransitionFee = documentsBatchTransitionFixture.calculateFee();

    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWith(
      documentsBatchTransitionFixture.getOwnerId(),
    );

    identity.reduceBalance(stateTransitionFee);

    expect(stateRepositoryMock.storeIdentity).to.be.calledOnceWith(identity);

    expect(blockExecutionStateMock.incrementAccumulativeFees).to.be.calledOnceWith(
      stateTransitionFee,
    );
  });

  it('should apply a DataContractCreateTransition, add it to block execution state and return ResponseDeliverTx', async () => {
    unserializeStateTransitionMock.resolves(dataContractCreateTransitionFixture);

    const response = await deliverTxHandler(dataContractRequest);

    expect(response).to.be.an.instanceOf(ResponseDeliverTx);
    expect(response.code).to.equal(0);

    expect(unserializeStateTransitionMock).to.be.calledOnceWith(
      dataContractCreateTransitionFixture.serialize(),
    );
    expect(dppMock.stateTransition.validateData).to.be.calledOnceWith(
      dataContractCreateTransitionFixture,
    );
    expect(dppMock.stateTransition.apply).to.be.calledOnceWith(
      dataContractCreateTransitionFixture,
    );
    expect(blockExecutionStateMock.addDataContract).to.be.calledOnceWith(
      dataContractCreateTransitionFixture.getDataContract(),
    );

    expect(blockExecutionStateMock.incrementAccumulativeFees).to.be.calledOnceWith(
      dataContractCreateTransitionFixture.calculateFee(),
    );
  });

  it('should throw InvalidArgumentAbciError if a state transition is not valid', async () => {
    unserializeStateTransitionMock.resolves(dataContractCreateTransitionFixture);

    const error = new ValidationError('Some error');
    const invalidResult = new ValidationResult([error]);

    dppMock.stateTransition.validateData.resolves(invalidResult);

    try {
      await deliverTxHandler(documentRequest);

      expect.fail('should throw InvalidArgumentAbciError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentAbciError);
      expect(e.getMessage()).to.equal('Invalid state transition');
      expect(e.getCode()).to.equal(AbciError.CODES.INVALID_ARGUMENT);
      expect(e.getData()).to.deep.equal({ errors: [error] });
      expect(blockExecutionStateMock.incrementAccumulativeFees).to.not.be.called();
    }
  });
});
