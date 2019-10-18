const validateDocumentsSTDataFactory = require('../../../../../lib/document/stateTransition/validation/data/validateDocumentsSTDataFactory');

const Document = require('../../../../../lib/document/Document');
const DocumentsStateTransition = require('../../../../../lib/document/stateTransition/DocumentsStateTransition');

const DataTriggerExecutionContext = require('../../../../../lib/dataTrigger/DataTriggerExecutionContext');
const DataTriggerExecutionError = require('../../../../../lib/errors/DataTriggerExecutionError');
const DataTriggerExecutionResult = require('../../../../../lib/dataTrigger/DataTriggerExecutionResult');

const getDocumentsFixture = require('../../../../../lib/test/fixtures/getDocumentsFixture');
const getContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');
const createDataProviderMock = require('../../../../../lib/test/mocks/createDataProviderMock');

const ValidationResult = require('../../../../../lib/validation/ValidationResult');

const { expectValidationError } = require('../../../../../lib/test/expect/expectError');

const DocumentAlreadyPresentError = require('../../../../../lib/errors/DocumentAlreadyPresentError');
const DocumentNotFoundError = require('../../../../../lib/errors/DocumentNotFoundError');
const InvalidDocumentRevisionError = require('../../../../../lib/errors/InvalidDocumentRevisionError');
const DataContractNotPresentError = require('../../../../../lib/errors/DataContractNotPresentError');
const ConsensusError = require('../../../../../lib/errors/ConsensusError');
const InvalidDocumentActionError = require('../../../../../lib/stPacket/errors/InvalidDocumentActionError');

describe('validateDocumentsSTDataFactory', () => {
  let validateDocumentsSTData;
  let fetchDocumentsMock;
  let stateTransition;
  let documents;
  let dataContract;
  let userId;
  let validateDocumentsUniquenessByIndicesMock;
  let dataProviderMock;
  let validateBlockchainUserMock;
  let executeDataTriggersMock;

  beforeEach(function beforeEach() {
    ({ userId } = getDocumentsFixture);

    documents = getDocumentsFixture();
    dataContract = getContractFixture();

    stateTransition = new DocumentsStateTransition(documents);


    dataProviderMock = createDataProviderMock(this.sinonSandbox);
    dataProviderMock.fetchDataContract.resolves(dataContract);

    fetchDocumentsMock = this.sinonSandbox.stub().resolves([]);
    validateBlockchainUserMock = this.sinonSandbox.stub().resolves(new ValidationResult());
    executeDataTriggersMock = this.sinonSandbox.stub();

    validateDocumentsUniquenessByIndicesMock = this.sinonSandbox.stub();
    validateDocumentsUniquenessByIndicesMock.resolves(new ValidationResult());

    validateDocumentsSTData = validateDocumentsSTDataFactory(
      dataProviderMock,
      validateBlockchainUserMock,
      fetchDocumentsMock,
      validateDocumentsUniquenessByIndicesMock,
      executeDataTriggersMock,
    );
  });

  it('should return invalid result if userId is not valid', async () => {
    const userError = new ConsensusError('error');

    validateBlockchainUserMock.resolves(new ValidationResult([userError]));

    const result = await validateDocumentsSTData(stateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(userError);

    expect(dataProviderMock.fetchDataContract).to.have.not.been.called();
    expect(fetchDocumentsMock).to.have.not.been.called();
    expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should return invalid result if Data Contract is not present', async () => {
    dataProviderMock.fetchDataContract.resolves(null);

    const result = await validateDocumentsSTData(stateTransition);

    expectValidationError(result, DataContractNotPresentError);

    const [error] = result.getErrors();

    expect(error.getDataContractId()).to.equal(dataContract.getId());

    expect(dataProviderMock.fetchDataContract).to.have.been.calledOnceWith(dataContract.getId());

    expect(fetchDocumentsMock).to.have.not.been.called();
    expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should return invalid result if Document with action "create" is already present', async () => {
    fetchDocumentsMock.resolves([documents[0]]);

    const result = await validateDocumentsSTData(stateTransition);

    expectValidationError(result, DocumentAlreadyPresentError);

    const [error] = result.getErrors();

    expect(error.getDocument()).to.equal(documents[0]);
    expect(error.getFetchedDocument()).to.equal(documents[0]);

    expect(dataProviderMock.fetchDataContract).to.have.been.calledOnceWith(dataContract.getId());

    expect(fetchDocumentsMock).to.have.been.calledOnceWith(documents);

    expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should return invalid result if Document with action "update" is not present', async () => {
    documents[0].setAction(Document.ACTIONS.REPLACE);

    const result = await validateDocumentsSTData(stateTransition);

    expectValidationError(result, DocumentNotFoundError);

    const [error] = result.getErrors();

    expect(error.getDocument()).to.equal(documents[0]);

    expect(dataProviderMock.fetchDataContract).to.have.been.calledOnceWith(dataContract.getId());

    expect(fetchDocumentsMock).to.have.been.calledOnceWith(documents);

    expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should return invalid result if Document with action "delete" is not present', async () => {
    documents[0].setData({});
    documents[0].setAction(Document.ACTIONS.DELETE);

    const result = await validateDocumentsSTData(stateTransition);

    expectValidationError(result, DocumentNotFoundError);

    const [error] = result.getErrors();

    expect(error.getDocument()).to.equal(documents[0]);

    expect(dataProviderMock.fetchDataContract).to.have.been.calledOnceWith(dataContract.getId());

    expect(fetchDocumentsMock).to.have.been.calledOnceWith(documents);

    expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should return invalid result if Document with action "update" has wrong revision', async () => {
    documents[0].setAction(Document.ACTIONS.REPLACE);

    fetchDocumentsMock.resolves([documents[0]]);

    const result = await validateDocumentsSTData(stateTransition);

    expectValidationError(result, InvalidDocumentRevisionError);

    const [error] = result.getErrors();

    expect(error.getDocument()).to.equal(documents[0]);
    expect(error.getFetchedDocument()).to.equal(documents[0]);

    expect(dataProviderMock.fetchDataContract).to.have.been.calledOnceWith(dataContract.getId());

    expect(fetchDocumentsMock).to.have.been.calledOnceWith(documents);

    expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should return invalid result if Document with action "delete" has wrong revision', async () => {
    documents[0].setData({});
    documents[0].setAction(Document.ACTIONS.DELETE);

    fetchDocumentsMock.resolves([documents[0]]);

    const result = await validateDocumentsSTData(stateTransition);

    expectValidationError(result, InvalidDocumentRevisionError);

    const [error] = result.getErrors();

    expect(error.getDocument()).to.equal(documents[0]);

    expect(dataProviderMock.fetchDataContract).to.have.been.calledOnceWith(dataContract.getId());

    expect(fetchDocumentsMock).to.have.been.calledOnceWith(documents);

    expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should throw an error if Document has invalid action', async () => {
    documents[0].setAction(5);

    fetchDocumentsMock.resolves([documents[0]]);

    try {
      await validateDocumentsSTData(stateTransition);

      expect.fail('InvalidDocumentActionError should be thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidDocumentActionError);
      expect(e.getDocument()).to.equal(documents[0]);

      expect(dataProviderMock.fetchDataContract).to.have.been.calledOnceWith(dataContract.getId());

      expect(fetchDocumentsMock).to.have.been.calledOnceWith(documents);

      expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
      expect(executeDataTriggersMock).to.have.not.been.called();
    }
  });

  it('should return invalid result if there are duplicate documents according to unique indices', async () => {
    const duplicateDocumentsError = new ConsensusError('error');

    validateDocumentsUniquenessByIndicesMock.resolves(
      new ValidationResult([duplicateDocumentsError]),
    );

    const result = await validateDocumentsSTData(stateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(duplicateDocumentsError);

    expect(dataProviderMock.fetchDataContract).to.have.been.calledOnceWith(dataContract.getId());

    expect(fetchDocumentsMock).to.have.been.calledOnceWith(documents);

    expect(validateDocumentsUniquenessByIndicesMock).to.have.been.calledOnceWith(
      documents,
      dataContract,
    );

    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should return invalid result if data triggers execution failed', async () => {
    const dataTriggersExecutionContext = new DataTriggerExecutionContext(
      dataProviderMock,
      userId,
      dataContract,
    );

    const dataTriggerExecutionError = new DataTriggerExecutionError(
      documents[0],
      dataTriggersExecutionContext,
      new Error('error'),
    );

    executeDataTriggersMock.resolves([
      new DataTriggerExecutionResult([dataTriggerExecutionError]),
    ]);

    const result = await validateDocumentsSTData(stateTransition);

    expectValidationError(result, DataTriggerExecutionError);

    const [error] = result.getErrors();

    expect(error).to.equal(dataTriggerExecutionError);

    expect(dataProviderMock.fetchDataContract).to.have.been.calledOnceWith(dataContract.getId());

    expect(fetchDocumentsMock).to.have.been.calledOnceWith(documents);

    expect(validateDocumentsUniquenessByIndicesMock).to.have.been.calledOnceWith(
      documents,
      dataContract,
    );

    expect(executeDataTriggersMock).to.have.been.calledOnceWith(
      documents,
      dataTriggersExecutionContext,
    );
  });

  it('should return valid result if Documents are valid', async () => {
    const fetchedDocuments = [
      new Document(documents[1].toJSON()),
      new Document(documents[2].toJSON()),
    ];

    fetchDocumentsMock.resolves(fetchedDocuments);

    documents[1].setAction(Document.ACTIONS.REPLACE);
    documents[1].setRevision(2);

    documents[2].setData({});
    documents[2].setAction(Document.ACTIONS.DELETE);
    documents[2].setRevision(2);

    const dataTriggersExecutionContext = new DataTriggerExecutionContext(
      dataProviderMock,
      userId,
      dataContract,
    );

    executeDataTriggersMock.resolves([
      new DataTriggerExecutionResult(),
    ]);


    const result = await validateDocumentsSTData(stateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(dataProviderMock.fetchDataContract).to.have.been.calledOnceWith(dataContract.getId());

    expect(fetchDocumentsMock).to.have.been.calledOnceWith(documents);

    expect(validateDocumentsUniquenessByIndicesMock).to.have.been.calledOnceWith(
      documents,
      dataContract,
    );

    expect(executeDataTriggersMock).to.have.been.calledOnceWith(
      documents,
      dataTriggersExecutionContext,
    );
  });
});
