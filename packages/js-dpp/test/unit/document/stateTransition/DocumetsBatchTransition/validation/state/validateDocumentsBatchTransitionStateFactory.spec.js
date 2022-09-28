const validateDocumentsBatchTransitionStateFactory = require('../../../../../../../lib/document/stateTransition/DocumentsBatchTransition/validation/state/validateDocumentsBatchTransitionStateFactory');

const Document = require('../../../../../../../lib/document/Document');
const DocumentsBatchTransition = require('../../../../../../../lib/document/stateTransition/DocumentsBatchTransition/DocumentsBatchTransition');

const DataTriggerExecutionContext = require('../../../../../../../lib/dataTrigger/DataTriggerExecutionContext');
const DataTriggerExecutionError = require('../../../../../../../lib/errors/consensus/state/dataContract/dataTrigger/DataTriggerExecutionError');
const DataTriggerExecutionResult = require('../../../../../../../lib/dataTrigger/DataTriggerExecutionResult');

const getDataContractFixture = require('../../../../../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../../../../../lib/test/fixtures/getDocumentsFixture');
const getDocumentTransitionsFixture = require('../../../../../../../lib/test/fixtures/getDocumentTransitionsFixture');
const createStateRepositoryMock = require('../../../../../../../lib/test/mocks/createStateRepositoryMock');

const ValidationResult = require('../../../../../../../lib/validation/ValidationResult');

const { expectValidationError } = require('../../../../../../../lib/test/expect/expectError');

const DataContractNotPresentError = require('../../../../../../../lib/errors/DataContractNotPresentError');

const DocumentAlreadyPresentError = require('../../../../../../../lib/errors/consensus/state/document/DocumentAlreadyPresentError');
const DocumentNotFoundError = require('../../../../../../../lib/errors/consensus/state/document/DocumentNotFoundError');
const InvalidDocumentRevisionError = require('../../../../../../../lib/errors/consensus/state/document/InvalidDocumentRevisionError');
const InvalidDocumentActionError = require('../../../../../../../lib/document/errors/InvalidDocumentActionError');
const DocumentOwnerIdMismatchError = require('../../../../../../../lib/errors/consensus/state/document/DocumentOwnerIdMismatchError');
const DocumentTimestampsMismatchError = require('../../../../../../../lib/errors/consensus/state/document/DocumentTimestampsMismatchError');
const DocumentTimestampWindowViolationError = require('../../../../../../../lib/errors/consensus/state/document/DocumentTimestampWindowViolationError');

const generateRandomIdentifier = require('../../../../../../../lib/test/utils/generateRandomIdentifier');
const SomeConsensusError = require('../../../../../../../lib/test/mocks/SomeConsensusError');
const StateTransitionExecutionContext = require('../../../../../../../lib/stateTransition/StateTransitionExecutionContext');

describe('validateDocumentsBatchTransitionStateFactory', () => {
  let validateDocumentsBatchTransitionState;
  let fetchDocumentsMock;
  let stateTransition;
  let documents;
  let dataContract;
  let ownerId;
  let validateDocumentsUniquenessByIndicesMock;
  let stateRepositoryMock;
  let executeDataTriggersMock;
  let documentTransitions;
  let abciHeader;
  let fakeTime;
  let blockTime;
  let executionContext;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();
    documents = getDocumentsFixture(dataContract);
    ownerId = getDocumentsFixture.ownerId;

    documentTransitions = getDocumentTransitionsFixture({
      create: documents,
    });

    stateTransition = new DocumentsBatchTransition({
      ownerId,
      transitions: documentTransitions.map((t) => t.toObject()),
    }, [dataContract]);

    executionContext = new StateTransitionExecutionContext();

    stateTransition.setExecutionContext(executionContext);

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchDataContract.resolves(dataContract);

    blockTime = new Date().getTime() / 1000;

    abciHeader = {
      time: {
        seconds: blockTime,
      },
    };

    stateRepositoryMock.fetchLatestPlatformBlockHeader.resolves(abciHeader);

    fetchDocumentsMock = this.sinonSandbox.stub().resolves([]);

    executeDataTriggersMock = this.sinonSandbox.stub();

    validateDocumentsUniquenessByIndicesMock = this.sinonSandbox.stub();
    validateDocumentsUniquenessByIndicesMock.resolves(new ValidationResult());

    validateDocumentsBatchTransitionState = validateDocumentsBatchTransitionStateFactory(
      stateRepositoryMock,
      fetchDocumentsMock,
      validateDocumentsUniquenessByIndicesMock,
      executeDataTriggersMock,
    );

    fakeTime = this.sinonSandbox.useFakeTimers(new Date());
  });

  afterEach(() => {
    fakeTime.reset();
  });

  it('should throw DataContractNotPresentError if data contract was not found', async () => {
    stateRepositoryMock.fetchDataContract.resolves(null);

    try {
      await validateDocumentsBatchTransitionState(stateTransition);

      expect.fail('should throw DataContractNotPresentError');
    } catch (e) {
      expect(e).to.be.instanceOf(DataContractNotPresentError);

      expect(e.getDataContractId()).to.deep.equal(dataContract.getId());

      expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
        dataContract.getId(),
        new StateTransitionExecutionContext(),
      );

      expect(fetchDocumentsMock).to.have.not.been.called();
      expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
      expect(executeDataTriggersMock).to.have.not.been.called();
    }
  });

  it('should return invalid result if document transition with action "create" is already present', async () => {
    fetchDocumentsMock.resolves([documents[0]]);

    const result = await validateDocumentsBatchTransitionState(stateTransition);

    expectValidationError(result, DocumentAlreadyPresentError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(4004);
    expect(Buffer.isBuffer(error.getDocumentId())).to.be.true();
    expect(error.getDocumentId()).to.deep.equal(documentTransitions[0].getId().toBuffer());

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContract.getId(),
      new StateTransitionExecutionContext(),
    );

    expect(fetchDocumentsMock.getCall(0).args[0].map((t) => t.toObject())).to.have.deep.members(
      documentTransitions.map((t) => t.toObject()),
    );

    expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should return invalid result if document transition with action "replace" is not present', async () => {
    documentTransitions = getDocumentTransitionsFixture({
      create: [],
      replace: [documents[0]],
    });

    stateTransition = new DocumentsBatchTransition({
      ownerId,
      contractId: dataContract.getId(),
      transitions: documentTransitions.map((t) => t.toObject()),
    }, [dataContract]);

    stateTransition.setExecutionContext(executionContext);

    const result = await validateDocumentsBatchTransitionState(stateTransition);

    expectValidationError(result, DocumentNotFoundError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(4005);
    expect(Buffer.isBuffer(error.getDocumentId())).to.be.true();
    expect(error.getDocumentId()).to.deep.equal(documentTransitions[0].getId().toBuffer());

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContract.getId(),
      new StateTransitionExecutionContext(),
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      documentTransitions,
      executionContext,
    );

    expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should return invalid result if document transition with action "delete" is not present', async () => {
    documentTransitions = getDocumentTransitionsFixture({
      create: [],
      delete: [documents[0]],
    });

    stateTransition = new DocumentsBatchTransition({
      ownerId,
      contractId: dataContract.getId(),
      transitions: documentTransitions.map((t) => t.toObject()),
    }, [dataContract]);

    stateTransition.setExecutionContext(executionContext);

    const result = await validateDocumentsBatchTransitionState(stateTransition);

    expectValidationError(result, DocumentNotFoundError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(4005);
    expect(Buffer.isBuffer(error.getDocumentId())).to.be.true();
    expect(error.getDocumentId()).to.deep.equal(documentTransitions[0].getId().toBuffer());

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContract.getId(),
      new StateTransitionExecutionContext(),
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      documentTransitions,
      executionContext,
    );

    expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should return invalid result if document transition with action "replace" has wrong revision', async () => {
    const replaceDocument = new Document(documents[0].toObject(), dataContract);
    replaceDocument.setRevision(3);

    documentTransitions = getDocumentTransitionsFixture({
      create: [],
      replace: [replaceDocument],
    });

    stateTransition = new DocumentsBatchTransition({
      ownerId,
      contractId: dataContract.getId(),
      transitions: documentTransitions.map((t) => t.toObject()),
    }, [dataContract]);

    stateTransition.setExecutionContext(executionContext);

    documents[0].setCreatedAt(replaceDocument.getCreatedAt());
    fetchDocumentsMock.resolves([documents[0]]);

    const result = await validateDocumentsBatchTransitionState(stateTransition);

    expectValidationError(result, InvalidDocumentRevisionError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(4010);
    expect(Buffer.isBuffer(error.getDocumentId())).to.be.true();
    expect(error.getDocumentId()).to.deep.equal(documentTransitions[0].getId().toBuffer());
    expect(error.getCurrentRevision()).to.deep.equal(documents[0].getRevision());

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContract.getId(),
      new StateTransitionExecutionContext(),
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      documentTransitions,
      executionContext,
    );

    expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should return invalid result if document transition with action "replace" has mismatch of ownerId with previous revision', async () => {
    const replaceDocument = new Document(documents[0].toObject(), dataContract);
    replaceDocument.setRevision(1);

    const fetchedDocument = new Document(documents[0].toObject(), dataContract);
    fetchedDocument.ownerId = generateRandomIdentifier();

    documentTransitions = getDocumentTransitionsFixture({
      create: [],
      replace: [replaceDocument],
    });

    stateTransition = new DocumentsBatchTransition({
      ownerId,
      contractId: dataContract.getId(),
      transitions: documentTransitions.map((t) => t.toObject()),
    }, [dataContract]);

    stateTransition.setExecutionContext(executionContext);

    fetchDocumentsMock.resolves([fetchedDocument]);

    const result = await validateDocumentsBatchTransitionState(stateTransition);

    expectValidationError(result, DocumentOwnerIdMismatchError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(4006);
    expect(Buffer.isBuffer(error.getDocumentId())).to.be.true();
    expect(error.getDocumentId()).to.deep.equal(documentTransitions[0].getId().toBuffer());

    expect(Buffer.isBuffer(error.getDocumentOwnerId())).to.be.true();
    expect(error.getDocumentOwnerId()).to.deep.equal(
      replaceDocument.getOwnerId().toBuffer(),
    );

    expect(Buffer.isBuffer(error.getExistingDocumentOwnerId())).to.be.true();
    expect(error.getExistingDocumentOwnerId()).to.deep.equal(
      fetchedDocument.getOwnerId().toBuffer(),
    );

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContract.getId(),
      new StateTransitionExecutionContext(),
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      documentTransitions,
      executionContext,
    );

    expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should throw an error if document transition has invalid action', async () => {
    stateTransition = new DocumentsBatchTransition({
      ownerId,
      contractId: dataContract.getId(),
      transitions: documentTransitions.map((t) => t.toObject()),
    }, [dataContract]);

    stateTransition.setExecutionContext(executionContext);

    stateTransition.transitions[0].getAction = () => 5;

    fetchDocumentsMock.resolves([documents[0]]);

    try {
      await validateDocumentsBatchTransitionState(stateTransition);

      expect.fail('InvalidDocumentActionError should be thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidDocumentActionError);
      expect(e.getDocumentTransition().toObject()).to.deep.equal(
        stateTransition.transitions[0].toObject(),
      );

      expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
        dataContract.getId(),
        new StateTransitionExecutionContext(),
      );

      expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
        stateTransition.transitions,
        executionContext,
      );

      expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
      expect(executeDataTriggersMock).to.have.not.been.called();
    }
  });

  it('should return invalid result if there are duplicate document transitions according to unique indices', async () => {
    const duplicateDocumentsError = new SomeConsensusError('error');

    validateDocumentsUniquenessByIndicesMock.resolves(
      new ValidationResult([duplicateDocumentsError]),
    );

    const result = await validateDocumentsBatchTransitionState(stateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(duplicateDocumentsError);

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContract.getId(),
      new StateTransitionExecutionContext(),
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      stateTransition.transitions,
      executionContext,
    );

    const [callOwnerId, callDocumentTransitions, callDataContract] = (
      validateDocumentsUniquenessByIndicesMock.getCall(0).args
    );

    const callArgs = [
      callOwnerId,
      callDocumentTransitions.map((t) => t.toObject()),
      callDataContract,
    ];

    expect(callArgs).to.have.deep.members([
      ownerId,
      documentTransitions.map((t) => t.toObject()),
      dataContract,
    ]);
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should return invalid result if data triggers execution failed', async () => {
    const dataTriggersExecutionContext = new DataTriggerExecutionContext(
      stateRepositoryMock,
      ownerId,
      dataContract,
      executionContext,
    );

    const dataTriggerExecutionError = new DataTriggerExecutionError(
      documentTransitions[0],
      dataTriggersExecutionContext.getDataContract(),
      dataTriggersExecutionContext.getOwnerId(),
      new Error('error'),
    );

    executeDataTriggersMock.resolves([
      new DataTriggerExecutionResult([dataTriggerExecutionError]),
    ]);

    const result = await validateDocumentsBatchTransitionState(stateTransition);

    expectValidationError(result, DataTriggerExecutionError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(4002);
    expect(error).to.equal(dataTriggerExecutionError);

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContract.getId(),
      new StateTransitionExecutionContext(),
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      stateTransition.transitions,
      executionContext,
    );

    const [callOwnerId, callDocumentTransitions, callDataContract, callExecutionContext] = (
      validateDocumentsUniquenessByIndicesMock.getCall(0).args
    );

    const callArgs = [
      callOwnerId,
      callDocumentTransitions.map((t) => t.toObject()),
      callDataContract,
      callExecutionContext,
    ];

    expect(callArgs).to.have.deep.members([
      ownerId,
      documentTransitions.map((t) => t.toObject()),
      dataContract,
      executionContext,
    ]);

    const [triggerCallDocumentTransitions, triggerCallDataTriggersExecutionContext] = (
      executeDataTriggersMock.getCall(0).args
    );

    const triggerCallArgs = [
      triggerCallDocumentTransitions.map((t) => t.toObject()),
      triggerCallDataTriggersExecutionContext,
    ];

    expect(triggerCallArgs).to.have.deep.members([
      documentTransitions.map((t) => t.toObject()),
      dataTriggersExecutionContext,
    ]);
  });

  describe('Timestamps', () => {
    let timeWindowStart;
    let timeWindowEnd;

    beforeEach(() => {
      timeWindowStart = new Date(blockTime * 1000);
      timeWindowStart.setMinutes(
        timeWindowStart.getMinutes() - 5,
      );

      timeWindowEnd = new Date(blockTime * 1000);
      timeWindowEnd.setMinutes(
        timeWindowEnd.getMinutes() + 5,
      );
    });

    describe('CREATE transition', () => {
      it('should return invalid result if timestamps mismatch', async () => {
        documentTransitions = getDocumentTransitionsFixture({
          create: [documents[0]],
        });

        stateTransition = new DocumentsBatchTransition({
          ownerId,
          contractId: dataContract.getId(),
          transitions: documentTransitions.map((t) => t.toObject()),
        }, [dataContract]);

        stateTransition.transitions.forEach((t) => {
          // eslint-disable-next-line no-param-reassign
          t.updatedAt = new Date();
        });

        const result = await validateDocumentsBatchTransitionState(stateTransition);

        expectValidationError(result, DocumentTimestampsMismatchError);

        const [error] = result.getErrors();

        expect(error.getCode()).to.equal(4007);

        documentTransitions[0].updatedAt = new Date();

        expect(Buffer.isBuffer(error.getDocumentId())).to.be.true();
        expect(error.getDocumentId()).to.deep.equal(documentTransitions[0].getId().toBuffer());
      });

      it('should return invalid result if "$createdAt" have violated time window', async () => {
        documentTransitions = getDocumentTransitionsFixture({
          create: [documents[0]],
        });

        stateTransition = new DocumentsBatchTransition({
          ownerId,
          contractId: dataContract.getId(),
          transitions: documentTransitions.map((t) => t.toObject()),
        }, [dataContract]);

        stateTransition.transitions.forEach((t) => {
          // eslint-disable-next-line no-param-reassign
          t.createdAt.setMinutes(t.createdAt.getMinutes() - 6);
          // eslint-disable-next-line no-param-reassign
          t.updatedAt = undefined;
        });

        const result = await validateDocumentsBatchTransitionState(stateTransition);

        expectValidationError(result, DocumentTimestampWindowViolationError);

        const [error] = result.getErrors();

        expect(error.getCode()).to.equal(4008);

        documentTransitions[0].createdAt.setMinutes(
          documentTransitions[0].createdAt.getMinutes() - 6,
        );
        documentTransitions[0].updatedAt = undefined;

        expect(Buffer.isBuffer(error.getDocumentId())).to.be.true();
        expect(error.getDocumentId()).to.deep.equal(documentTransitions[0].getId().toBuffer());
        expect(error.getTimestampName()).to.equal('createdAt');
        expect(error.getTimestamp()).to.deep.equal(documentTransitions[0].createdAt);
        expect(error.getTimeWindowStart()).to.deep.equal(timeWindowStart);
        expect(error.getTimeWindowEnd()).to.deep.equal(timeWindowEnd);
      });

      it('should return invalid result if "$updatedAt" have violated time window', async () => {
        documentTransitions = getDocumentTransitionsFixture({
          create: [documents[1]],
        });

        stateTransition = new DocumentsBatchTransition({
          ownerId,
          contractId: dataContract.getId(),
          transitions: documentTransitions.map((t) => t.toObject()),
        }, [dataContract]);

        stateTransition.transitions.forEach((t) => {
          // eslint-disable-next-line no-param-reassign
          t.updatedAt.setMinutes(t.updatedAt.getMinutes() - 6);
          // eslint-disable-next-line no-param-reassign
          t.createdAt = undefined;
        });

        const result = await validateDocumentsBatchTransitionState(stateTransition);

        expectValidationError(result, DocumentTimestampWindowViolationError);

        const [error] = result.getErrors();

        expect(error.getCode()).to.equal(4008);

        documentTransitions[0].updatedAt.setMinutes(
          documentTransitions[0].updatedAt.getMinutes() - 6,
        );
        documentTransitions[0].createdAt = undefined;

        expect(Buffer.isBuffer(error.getDocumentId())).to.be.true();
        expect(error.getDocumentId()).to.deep.equal(documentTransitions[0].getId().toBuffer());
        expect(error.getTimestampName()).to.equal('updatedAt');
        expect(error.getTimestamp()).to.deep.equal(documentTransitions[0].updatedAt);
        expect(error.getTimeWindowStart()).to.deep.equal(timeWindowStart);
        expect(error.getTimeWindowEnd()).to.deep.equal(timeWindowEnd);
      });

      it('should not validate time in block window on dry run', async () => {
        documentTransitions = getDocumentTransitionsFixture({
          create: [documents[1]],
        });

        executeDataTriggersMock.resolves([
          new DataTriggerExecutionResult(),
        ]);

        stateTransition = new DocumentsBatchTransition({
          ownerId,
          contractId: dataContract.getId(),
          transitions: documentTransitions.map((t) => t.toObject()),
        }, [dataContract]);

        stateTransition.transitions.forEach((t) => {
          // eslint-disable-next-line no-param-reassign
          t.updatedAt.setMinutes(t.updatedAt.getMinutes() - 6);
        });

        stateTransition.getExecutionContext().enableDryRun();

        const result = await validateDocumentsBatchTransitionState(stateTransition);

        stateTransition.getExecutionContext().disableDryRun();

        expect(result).to.be.an.instanceOf(ValidationResult);
        expect(result.isValid()).to.be.true();
      });

      it('should return valid result if timestamps mismatch on dry run', async () => {
        documentTransitions = getDocumentTransitionsFixture({
          create: [documents[0]],
        });

        executeDataTriggersMock.resolves([
          new DataTriggerExecutionResult(),
        ]);

        stateTransition = new DocumentsBatchTransition({
          ownerId,
          contractId: dataContract.getId(),
          transitions: documentTransitions.map((t) => t.toObject()),
        }, [dataContract]);

        stateTransition.transitions.forEach((t) => {
          // eslint-disable-next-line no-param-reassign
          t.updatedAt = new Date();
        });

        stateTransition.getExecutionContext().enableDryRun();

        const result = await validateDocumentsBatchTransitionState(stateTransition);

        stateTransition.getExecutionContext().disableDryRun();

        expect(result).to.be.an.instanceOf(ValidationResult);
        expect(result.isValid()).to.be.true();
      });
    });

    describe('REPLACE transition', () => {
      it('should return invalid result if documents with action "replace" have violated time window', async () => {
        documentTransitions = getDocumentTransitionsFixture({
          create: [],
          replace: [documents[1]],
        });

        stateTransition = new DocumentsBatchTransition({
          ownerId,
          contractId: dataContract.getId(),
          transitions: documentTransitions.map((t) => t.toObject()),
        }, [dataContract]);

        documents[1].updatedAt.setMinutes(
          documents[1].updatedAt.getMinutes() - 6,
        );

        fetchDocumentsMock.resolves([documents[1]]);

        stateTransition.transitions.forEach((t) => {
          // eslint-disable-next-line no-param-reassign
          t.updatedAt.setMinutes(t.updatedAt.getMinutes() - 6);
        });

        const result = await validateDocumentsBatchTransitionState(stateTransition);

        expectValidationError(result, DocumentTimestampWindowViolationError);

        const [error] = result.getErrors();

        expect(error.getCode()).to.equal(4008);

        documentTransitions[0].updatedAt.setMinutes(
          documentTransitions[0].updatedAt.getMinutes() - 6,
        );

        expect(Buffer.isBuffer(error.getDocumentId())).to.be.true();
        expect(error.getDocumentId()).to.deep.equal(documentTransitions[0].getId().toBuffer());
        expect(error.getTimestampName()).to.equal('updatedAt');
        expect(error.getTimestamp()).to.deep.equal(documentTransitions[0].updatedAt);
        expect(error.getTimeWindowStart()).to.deep.equal(timeWindowStart);
        expect(error.getTimeWindowEnd()).to.deep.equal(timeWindowEnd);
      });

      it('should return valid result if documents with action "replace" have violated time window on dry run', async () => {
        executeDataTriggersMock.resolves([
          new DataTriggerExecutionResult(),
        ]);

        documentTransitions = getDocumentTransitionsFixture({
          create: [],
          replace: [documents[1]],
        });

        stateTransition = new DocumentsBatchTransition({
          ownerId,
          contractId: dataContract.getId(),
          transitions: documentTransitions.map((t) => t.toObject()),
        }, [dataContract]);

        documents[1].updatedAt.setMinutes(
          documents[1].updatedAt.getMinutes() - 6,
        );

        fetchDocumentsMock.resolves([documents[1]]);

        stateTransition.transitions.forEach((t) => {
          // eslint-disable-next-line no-param-reassign
          t.updatedAt.setMinutes(t.updatedAt.getMinutes() - 6);
        });

        stateTransition.getExecutionContext().enableDryRun();

        const result = await validateDocumentsBatchTransitionState(stateTransition);

        stateTransition.getExecutionContext().disableDryRun();

        expect(result).to.be.an.instanceOf(ValidationResult);
        expect(result.isValid()).to.be.true();
      });
    });
  });

  it('should return valid result if document transitions are valid', async () => {
    const fetchedDocuments = [
      new Document(documents[1].toObject(), dataContract),
      new Document(documents[2].toObject(), dataContract),
    ];

    fetchDocumentsMock.resolves(fetchedDocuments);

    documents[1].setRevision(1);
    documents[2].setRevision(1);

    documentTransitions = getDocumentTransitionsFixture({
      create: [],
      replace: [documents[1]],
      delete: [documents[2]],
    });

    stateTransition = new DocumentsBatchTransition({
      ownerId,
      contractId: dataContract.getId(),
      transitions: documentTransitions.map((t) => t.toObject()),
    }, [dataContract]);

    stateTransition.setExecutionContext(executionContext);

    const dataTriggersExecutionContext = new DataTriggerExecutionContext(
      stateRepositoryMock,
      ownerId,
      dataContract,
      executionContext,
    );

    executeDataTriggersMock.resolves([
      new DataTriggerExecutionResult(),
    ]);

    const result = await validateDocumentsBatchTransitionState(stateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContract.getId(),
      new StateTransitionExecutionContext(),
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      stateTransition.transitions,
      executionContext,
    );

    expect(validateDocumentsUniquenessByIndicesMock).to.have.been.calledOnceWithExactly(
      ownerId,
      [documentTransitions[0]],
      dataContract,
      executionContext,
    );

    expect(executeDataTriggersMock).to.have.been.calledOnceWithExactly(
      documentTransitions,
      dataTriggersExecutionContext,
    );
  });
});
