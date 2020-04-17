const validateDocumentsBatchTransitionDataFactory = require('../../../../../lib/document/stateTransition/validation/data/validateDocumentsBatchTransitionDataFactory');

const Document = require('../../../../../lib/document/Document');
const DocumentsBatchTransition = require('../../../../../lib/document/stateTransition/DocumentsBatchTransition');

const DataTriggerExecutionContext = require('../../../../../lib/dataTrigger/DataTriggerExecutionContext');
const DataTriggerExecutionError = require('../../../../../lib/errors/DataTriggerExecutionError');
const DataTriggerExecutionResult = require('../../../../../lib/dataTrigger/DataTriggerExecutionResult');

const getDocumentsFixture = require('../../../../../lib/test/fixtures/getDocumentsFixture');
const getDocumentTransitionsFixture = require('../../../../../lib/test/fixtures/getDocumentTransitionsFixture');
const createStateRepositoryMock = require('../../../../../lib/test/mocks/createStateRepositoryMock');

const ValidationResult = require('../../../../../lib/validation/ValidationResult');

const { expectValidationError } = require('../../../../../lib/test/expect/expectError');

const DataContractNotPresentError = require('../../../../../lib/errors/DataContractNotPresentError');
const DocumentAlreadyPresentError = require('../../../../../lib/errors/DocumentAlreadyPresentError');
const DocumentNotFoundError = require('../../../../../lib/errors/DocumentNotFoundError');
const InvalidDocumentRevisionError = require('../../../../../lib/errors/InvalidDocumentRevisionError');
const ConsensusError = require('../../../../../lib/errors/ConsensusError');
const InvalidDocumentActionError = require('../../../../../lib/document/errors/InvalidDocumentActionError');
const DocumentOwnerIdMismatchError = require('../../../../../lib/errors/DocumentOwnerIdMismatchError');

const generateRandomId = require('../../../../../lib/test/utils/generateRandomId');

describe('validateDocumentsBatchTransitionDataFactory', () => {
  let validateData;
  let fetchDocumentsMock;
  let stateTransition;
  let documents;
  let dataContract;
  let ownerId;
  let validateDocumentsUniquenessByIndicesMock;
  let stateRepositoryMock;
  let executeDataTriggersMock;
  let documentTransitions;

  beforeEach(function beforeEach() {
    documents = getDocumentsFixture();
    dataContract = getDocumentsFixture.dataContract;
    ownerId = getDocumentsFixture.ownerId;

    documentTransitions = getDocumentTransitionsFixture({
      create: documents,
    });

    stateTransition = new DocumentsBatchTransition({
      ownerId,
      contractId: dataContract.getId(),
      transitions: documentTransitions.map((t) => t.toJSON()),
    });

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchDataContract.resolves(dataContract);

    fetchDocumentsMock = this.sinonSandbox.stub().resolves([]);

    executeDataTriggersMock = this.sinonSandbox.stub();

    validateDocumentsUniquenessByIndicesMock = this.sinonSandbox.stub();
    validateDocumentsUniquenessByIndicesMock.resolves(new ValidationResult());

    validateData = validateDocumentsBatchTransitionDataFactory(
      stateRepositoryMock,
      fetchDocumentsMock,
      validateDocumentsUniquenessByIndicesMock,
      executeDataTriggersMock,
    );
  });

  it('should return invalid result if data contract was not found', async () => {
    stateRepositoryMock.fetchDataContract.resolves(undefined);

    const result = await validateData(stateTransition);

    expectValidationError(result, DataContractNotPresentError);

    const [error] = result.getErrors();

    expect(error.getDataContractId()).to.equal(dataContract.getId());

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContract.getId(),
    );

    expect(fetchDocumentsMock).to.have.not.been.called();
    expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should return invalid result if document transition with action "create" is already present', async () => {
    fetchDocumentsMock.resolves([documents[0]]);

    const result = await validateData(stateTransition);

    expectValidationError(result, DocumentAlreadyPresentError);

    const [error] = result.getErrors();

    expect(error.getDocumentTransition()).to.deep.equal(documentTransitions[0]);
    expect(error.getFetchedDocument()).to.deep.equal(documents[0]);

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContract.getId(),
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      dataContract.getId(), documentTransitions,
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
      transitions: documentTransitions.map((t) => t.toJSON()),
    });

    const result = await validateData(stateTransition);

    expectValidationError(result, DocumentNotFoundError);

    const [error] = result.getErrors();

    expect(error.getDocumentTransition()).to.deep.equal(documentTransitions[0]);

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContract.getId(),
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      dataContract.getId(), documentTransitions,
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
      transitions: documentTransitions.map((t) => t.toJSON()),
    });

    const result = await validateData(stateTransition);

    expectValidationError(result, DocumentNotFoundError);

    const [error] = result.getErrors();

    expect(error.getDocumentTransition()).to.deep.equal(documentTransitions[0]);

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContract.getId(),
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      dataContract.getId(), documentTransitions,
    );

    expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should return invalid result if document transition with action "replace" has wrong revision', async () => {
    const replaceDocument = new Document(documents[0].toJSON());
    replaceDocument.setRevision(3);

    documentTransitions = getDocumentTransitionsFixture({
      create: [],
      replace: [replaceDocument],
    });

    stateTransition = new DocumentsBatchTransition({
      ownerId,
      contractId: dataContract.getId(),
      transitions: documentTransitions.map((t) => t.toJSON()),
    });

    fetchDocumentsMock.resolves([documents[0]]);

    const result = await validateData(stateTransition);

    expectValidationError(result, InvalidDocumentRevisionError);

    const [error] = result.getErrors();

    expect(error.getDocumentTransition()).to.deep.equal(documentTransitions[0]);
    expect(error.getFetchedDocument()).to.deep.equal(documents[0]);

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContract.getId(),
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      dataContract.getId(), documentTransitions,
    );

    expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should return invalid result if document transition with action "replace" has mismatch of ownerId with previous revision', async () => {
    const replaceDocument = new Document(documents[0].toJSON());
    replaceDocument.setRevision(1);

    const fetchedDocument = new Document(documents[0].toJSON());
    fetchedDocument.ownerId = generateRandomId();

    documentTransitions = getDocumentTransitionsFixture({
      create: [],
      replace: [replaceDocument],
    });

    stateTransition = new DocumentsBatchTransition({
      ownerId,
      contractId: dataContract.getId(),
      transitions: documentTransitions.map((t) => t.toJSON()),
    });

    fetchDocumentsMock.resolves([fetchedDocument]);

    const result = await validateData(stateTransition);

    expectValidationError(result, DocumentOwnerIdMismatchError);

    const [error] = result.getErrors();

    expect(error.getDocumentTransition()).to.deep.equal(documentTransitions[0]);
    expect(error.getFetchedDocument()).to.deep.equal(fetchedDocument);

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContract.getId(),
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      dataContract.getId(), documentTransitions,
    );

    expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should throw an error if document transition has invalid action', async () => {
    stateTransition = new DocumentsBatchTransition({
      ownerId,
      contractId: dataContract.getId(),
      transitions: documentTransitions.map((t) => t.toJSON()),
    });

    stateTransition.transitions[0].getAction = () => 5;

    fetchDocumentsMock.resolves([documents[0]]);

    try {
      await validateData(stateTransition);

      expect.fail('InvalidDocumentActionError should be thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidDocumentActionError);
      expect(e.getDocumentTransition().toJSON()).to.deep.equal(
        stateTransition.transitions[0].toJSON(),
      );

      expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
        dataContract.getId(),
      );

      expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
        dataContract.getId(), stateTransition.transitions,
      );

      expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
      expect(executeDataTriggersMock).to.have.not.been.called();
    }
  });

  it('should return invalid result if there are duplicate document transitions according to unique indices', async () => {
    const duplicateDocumentsError = new ConsensusError('error');

    validateDocumentsUniquenessByIndicesMock.resolves(
      new ValidationResult([duplicateDocumentsError]),
    );

    const result = await validateData(stateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(duplicateDocumentsError);

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContract.getId(),
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      dataContract.getId(), stateTransition.transitions,
    );

    expect(validateDocumentsUniquenessByIndicesMock).to.have.been.calledOnceWithExactly(
      ownerId,
      documentTransitions,
      dataContract,
    );
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should return invalid result if data triggers execution failed', async () => {
    const dataTriggersExecutionContext = new DataTriggerExecutionContext(
      stateRepositoryMock,
      ownerId,
      dataContract,
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

    const result = await validateData(stateTransition);

    expectValidationError(result, DataTriggerExecutionError);

    const [error] = result.getErrors();

    expect(error).to.equal(dataTriggerExecutionError);

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContract.getId(),
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      dataContract.getId(), stateTransition.transitions,
    );

    expect(validateDocumentsUniquenessByIndicesMock).to.have.been.calledOnceWithExactly(
      ownerId,
      documentTransitions,
      dataContract,
    );

    expect(executeDataTriggersMock).to.have.been.calledOnceWithExactly(
      documentTransitions,
      dataTriggersExecutionContext,
    );
  });

  it('should return valid result if document transitions are valid', async () => {
    const fetchedDocuments = [
      new Document(documents[1].toJSON()),
      new Document(documents[2].toJSON()),
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
      transitions: documentTransitions.map((t) => t.toJSON()),
    });

    const dataTriggersExecutionContext = new DataTriggerExecutionContext(
      stateRepositoryMock,
      ownerId,
      dataContract,
    );

    executeDataTriggersMock.resolves([
      new DataTriggerExecutionResult(),
    ]);

    const result = await validateData(stateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContract.getId(),
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      dataContract.getId(), stateTransition.transitions,
    );

    expect(validateDocumentsUniquenessByIndicesMock).to.have.been.calledOnceWithExactly(
      ownerId,
      [documentTransitions[0]],
      dataContract,
    );

    expect(executeDataTriggersMock).to.have.been.calledOnceWithExactly(
      documentTransitions,
      dataTriggersExecutionContext,
    );
  });
});
