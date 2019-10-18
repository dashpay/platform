const DocumentsStateTransition = require('../../../../../lib/document/stateTransition/DocumentsStateTransition');

const getContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../../../lib/test/fixtures/getDocumentsFixture');

const createDataProviderMock = require('../../../../../lib/test/mocks/createDataProviderMock');

const ValidationResult = require('../../../../../lib/validation/ValidationResult');

const validateDocumentsSTStructureFactory = require('../../../../../lib/document/stateTransition/validation/structure/validateDocumentsSTStructureFactory');

const { expectValidationError } = require('../../../../../lib/test/expect/expectError');

const DuplicateDocumentsError = require('../../../../../lib/errors/STDuplicateDocumentsError');
const MismatchSTDocumentsAndActionsError = require('../../../../../lib/errors/MismatchSTDocumentsAndActionsError');
const STContainsDocumentsFromDifferentUsersError = require('../../../../../lib/errors/STContainsDocumentsFromDifferentUsersError');
const ConsensusError = require('../../../../../lib/errors/ConsensusError');
const MissingDocumentContractIdError = require('../../../../../lib/errors/MissingDocumentContractIdError');
const STContainsDocumentsForDifferentDataContractsError = require('../../../../../lib/errors/STContainsDocumentsForDifferentDataContractsError');

describe('validateDocumentsSTStructureFactory', () => {
  let dataContract;
  let documents;
  let rawStateTransition;
  let findDuplicateDocumentsByIdMock;
  let findDuplicateDocumentsByIndicesMock;
  let validateDocumentMock;
  let validateDocumentsSTStructure;
  let dataProviderMock;

  beforeEach(function beforeEach() {
    dataContract = getContractFixture();
    documents = getDocumentsFixture();
    const stateTransition = new DocumentsStateTransition(documents);
    rawStateTransition = stateTransition.toJSON();

    findDuplicateDocumentsByIdMock = this.sinonSandbox.stub().returns([]);
    findDuplicateDocumentsByIndicesMock = this.sinonSandbox.stub().returns([]);
    validateDocumentMock = this.sinonSandbox.stub().returns(new ValidationResult());

    dataProviderMock = createDataProviderMock(this.sinonSandbox);
    dataProviderMock.fetchDataContract.resolves(dataContract);

    validateDocumentsSTStructure = validateDocumentsSTStructureFactory(
      validateDocumentMock,
      findDuplicateDocumentsByIdMock,
      findDuplicateDocumentsByIndicesMock,
      dataProviderMock,
    );
  });

  it('should return invalid result if actions and documents count are not equal', async () => {
    rawStateTransition.actions.push(3);

    const result = await validateDocumentsSTStructure(rawStateTransition);

    expectValidationError(result, MismatchSTDocumentsAndActionsError);

    const [error] = result.getErrors();

    expect(error.getRawStateTransition()).to.equal(rawStateTransition);

    expect(validateDocumentMock).to.not.be.called();
    expect(findDuplicateDocumentsByIdMock).to.not.be.called();
    expect(findDuplicateDocumentsByIndicesMock).to.not.be.called();
  });

  it('should return invalid result if documents do not contain $contractId', async () => {
    const secondRawDocument = rawStateTransition.documents[1];
    delete secondRawDocument.$contractId;

    const result = await validateDocumentsSTStructure(rawStateTransition);

    expectValidationError(result, MissingDocumentContractIdError);

    const [error] = result.getErrors();

    expect(error.getRawDocument()).to.equal(secondRawDocument);

    expect(validateDocumentMock).to.not.be.called();
    expect(findDuplicateDocumentsByIdMock).to.not.be.called();
    expect(findDuplicateDocumentsByIndicesMock).to.not.be.called();
  });

  it('should return invalid result if there are documents with different $contractId', async () => {
    const [firstRawDocument, secondRawDocument] = rawStateTransition.documents;
    secondRawDocument.$contractId = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

    const result = await validateDocumentsSTStructure(rawStateTransition);

    expectValidationError(result, STContainsDocumentsForDifferentDataContractsError);

    const [error] = result.getErrors();

    expect(error.getRawDocuments()).to.deep.equal([firstRawDocument, secondRawDocument]);

    expect(validateDocumentMock).to.not.be.called();
    expect(findDuplicateDocumentsByIdMock).to.not.be.called();
    expect(findDuplicateDocumentsByIndicesMock).to.not.be.called();
  });

  it('should return invalid result if Documents are invalid', async () => {
    const documentError = new ConsensusError('test');

    validateDocumentMock.onCall(0).returns(
      new ValidationResult([documentError]),
    );

    const result = await validateDocumentsSTStructure(rawStateTransition);

    expectValidationError(result, ConsensusError, 1);

    const [error] = result.getErrors();

    expect(error).to.equal(documentError);

    expect(validateDocumentMock.callCount).to.equal(5);

    documents.forEach((document) => {
      expect(validateDocumentMock).to.have.been.calledWith(
        document.toJSON(),
        dataContract,
        { action: document.getAction() },
      );
    });

    expect(findDuplicateDocumentsByIdMock).to.not.be.called();
    expect(findDuplicateDocumentsByIndicesMock).to.not.be.called();
  });

  it('should return invalid result if there are duplicate Documents with the same ID', async () => {
    const duplicateDocuments = [documents[0].toJSON()];

    findDuplicateDocumentsByIdMock.returns(duplicateDocuments);

    const result = await validateDocumentsSTStructure(rawStateTransition);

    expectValidationError(result, DuplicateDocumentsError);

    const [error] = result.getErrors();

    expect(error.getDuplicatedDocuments()).to.deep.equal(duplicateDocuments);

    expect(validateDocumentMock.callCount).to.equal(5);

    documents.forEach((document) => {
      expect(validateDocumentMock).to.have.been.calledWith(
        document.toJSON(),
        dataContract,
        { action: document.getAction() },
      );
    });

    expect(findDuplicateDocumentsByIdMock).to.be.called(documents);
    expect(findDuplicateDocumentsByIndicesMock).to.be.calledOnceWith(documents, dataContract);
  });

  it('should return invalid result if there are duplicate unique index values', async () => {
    const duplicateDocuments = [documents[1].toJSON()];

    findDuplicateDocumentsByIndicesMock.returns(duplicateDocuments);

    const result = await validateDocumentsSTStructure(rawStateTransition);

    expectValidationError(result, DuplicateDocumentsError);

    const [error] = result.getErrors();

    expect(error.getDuplicatedDocuments()).to.deep.equal(duplicateDocuments);

    expect(validateDocumentMock.callCount).to.equal(5);

    documents.forEach((document) => {
      expect(validateDocumentMock).to.have.been.calledWith(
        document.toJSON(),
        dataContract,
        { action: document.getAction() },
      );
    });

    expect(findDuplicateDocumentsByIdMock).to.have.been.calledOnceWith(documents);
    expect(findDuplicateDocumentsByIndicesMock).to.be.calledOnceWith(documents, dataContract);
  });

  it('should return invalid result if there are documents with different User IDs', async () => {
    const differentUserId = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

    documents[0].userId = differentUserId;
    rawStateTransition.documents[0].$userId = differentUserId;

    const result = await validateDocumentsSTStructure(rawStateTransition);

    expectValidationError(result, STContainsDocumentsFromDifferentUsersError);

    const [error] = result.getErrors();

    expect(error.getRawDocuments()).to.deep.equal([
      documents[0].toJSON(),
      documents[1].toJSON(),
    ]);

    expect(validateDocumentMock.callCount).to.equal(5);

    documents.forEach((document) => {
      expect(validateDocumentMock).to.have.been.calledWith(
        document.toJSON(),
        dataContract,
        { action: document.getAction() },
      );
    });

    expect(findDuplicateDocumentsByIdMock).to.have.been.calledOnceWith(documents);
    expect(findDuplicateDocumentsByIndicesMock).to.be.calledOnceWith(documents, dataContract);
  });

  it('should return valid result', async () => {
    const result = await validateDocumentsSTStructure(rawStateTransition, dataContract);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(findDuplicateDocumentsByIdMock).to.have.been.calledOnceWith(documents);

    expect(validateDocumentMock.callCount).to.equal(5);

    documents.forEach((document) => {
      expect(validateDocumentMock).to.have.been.calledWith(
        document.toJSON(),
        dataContract,
        { action: document.getAction() },
      );
    });

    expect(findDuplicateDocumentsByIdMock).to.have.been.calledOnceWith(documents);
    expect(findDuplicateDocumentsByIndicesMock).to.be.calledOnceWith(documents, dataContract);
  });
});
