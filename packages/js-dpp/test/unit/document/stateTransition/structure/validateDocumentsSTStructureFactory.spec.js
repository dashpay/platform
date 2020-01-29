const generateRandomId = require('../../../../../lib/test/utils/generateRandomId');

const DocumentsStateTransition = require('../../../../../lib/document/stateTransition/DocumentsStateTransition');

const getContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../../../lib/test/fixtures/getDocumentsFixture');

const ValidationResult = require('../../../../../lib/validation/ValidationResult');

const validateDocumentsSTStructureFactory = require('../../../../../lib/document/stateTransition/validation/structure/validateDocumentsSTStructureFactory');

const { expectValidationError } = require('../../../../../lib/test/expect/expectError');

const DuplicateDocumentsError = require('../../../../../lib/errors/STDuplicateDocumentsError');
const MismatchSTDocumentsAndActionsError = require('../../../../../lib/errors/MismatchSTDocumentsAndActionsError');
const STContainsDocumentsFromDifferentUsersError = require('../../../../../lib/errors/STContainsDocumentsFromDifferentUsersError');
const ConsensusError = require('../../../../../lib/errors/ConsensusError');
const STContainsDocumentsForDifferentDataContractsError = require('../../../../../lib/errors/STContainsDocumentsForDifferentDataContractsError');
const InvalidIdentityPublicKeyTypeError = require('../../../../../lib/errors/InvalidIdentityPublicKeyTypeError');

const Identity = require('../../../../../lib/identity/Identity');

describe('validateDocumentsSTStructureFactory', () => {
  let dataContract;
  let documents;
  let rawStateTransition;
  let findDuplicateDocumentsByIdMock;
  let findDuplicateDocumentsByIndicesMock;
  let validateDocumentMock;
  let validateDocumentsSTStructure;
  let fetchAndValidateDataContractMock;
  let stateTransition;
  let validateStateTransitionSignatureMock;
  let userId;
  let validateIdentityExistenceAndTypeMock;

  beforeEach(function beforeEach() {
    dataContract = getContractFixture();
    documents = getDocumentsFixture();
    stateTransition = new DocumentsStateTransition(documents);
    rawStateTransition = stateTransition.toJSON();

    findDuplicateDocumentsByIdMock = this.sinonSandbox.stub().returns([]);
    findDuplicateDocumentsByIndicesMock = this.sinonSandbox.stub().returns([]);
    validateDocumentMock = this.sinonSandbox.stub().returns(new ValidationResult());

    const dataContractValidationResult = new ValidationResult();
    dataContractValidationResult.setData(dataContract);

    fetchAndValidateDataContractMock = this.sinonSandbox.stub()
      .resolves(dataContractValidationResult);

    const validateSignatureResult = new ValidationResult();
    validateStateTransitionSignatureMock = this.sinonSandbox.stub().resolves(
      validateSignatureResult,
    );

    ([{ userId }] = documents);

    validateIdentityExistenceAndTypeMock = this.sinonSandbox.stub().resolves(
      new ValidationResult(),
    );

    validateDocumentsSTStructure = validateDocumentsSTStructureFactory(
      validateDocumentMock,
      findDuplicateDocumentsByIdMock,
      findDuplicateDocumentsByIndicesMock,
      fetchAndValidateDataContractMock,
      validateStateTransitionSignatureMock,
      validateIdentityExistenceAndTypeMock,
    );
  });

  it('should return invalid result if userId is not valid', async () => {
    const userError = new ConsensusError('error');

    validateIdentityExistenceAndTypeMock.resolves(new ValidationResult([userError]));

    const result = await validateDocumentsSTStructure(rawStateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(validateIdentityExistenceAndTypeMock).to.be.calledOnceWithExactly(
      userId, [Identity.TYPES.USER, Identity.TYPES.APPLICATION],
    );

    expect(error).to.equal(userError);

    expect(findDuplicateDocumentsByIdMock).to.be.called(documents);
    expect(findDuplicateDocumentsByIndicesMock).to.be.calledOnceWith(documents, dataContract);
    expect(validateStateTransitionSignatureMock).to.be.not.called(stateTransition, userId);
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
    expect(validateStateTransitionSignatureMock).to.not.be.called();
    expect(validateIdentityExistenceAndTypeMock).to.not.be.called();
  });

  it('should return invalid result if there are documents with different $contractId', async () => {
    const [firstRawDocument, secondRawDocument, thirdRawDocument] = rawStateTransition.documents;

    secondRawDocument.$contractId = generateRandomId();
    delete thirdRawDocument.$contractId;

    const result = await validateDocumentsSTStructure(rawStateTransition);

    expectValidationError(result, STContainsDocumentsForDifferentDataContractsError);

    const [error] = result.getErrors();

    expect(error.getRawDocuments()).to.deep.equal([
      firstRawDocument,
      secondRawDocument,
      thirdRawDocument,
    ]);

    expect(fetchAndValidateDataContractMock).to.not.be.called();
    expect(validateDocumentMock).to.not.be.called();
    expect(findDuplicateDocumentsByIdMock).to.not.be.called();
    expect(findDuplicateDocumentsByIndicesMock).to.not.be.called();
    expect(validateStateTransitionSignatureMock).to.not.be.called();
    expect(validateIdentityExistenceAndTypeMock).to.not.be.called();
  });

  it('should return invalid result if Documents are invalid', async () => {
    const dataContractError = new ConsensusError('error');
    const dataContractValidationResult = new ValidationResult([
      dataContractError,
    ]);

    fetchAndValidateDataContractMock.resolves(dataContractValidationResult);

    const result = await validateDocumentsSTStructure(rawStateTransition);

    expectValidationError(result, ConsensusError, 1);

    const [error] = result.getErrors();

    expect(error).to.equal(dataContractError);

    expect(fetchAndValidateDataContractMock).to.be.calledOnceWith(documents[0].toJSON());
    expect(validateDocumentMock).to.not.be.called();
    expect(findDuplicateDocumentsByIdMock).to.not.be.called();
    expect(findDuplicateDocumentsByIndicesMock).to.not.be.called();
    expect(validateStateTransitionSignatureMock).to.not.be.called();
    expect(validateIdentityExistenceAndTypeMock).to.not.be.called();
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

    expect(fetchAndValidateDataContractMock).to.be.calledOnceWith(documents[0].toJSON());
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
    expect(validateStateTransitionSignatureMock).to.not.be.called();
    expect(validateIdentityExistenceAndTypeMock).to.not.be.called();
  });

  it('should return invalid result if there are duplicate Documents with the same ID', async () => {
    const duplicateDocuments = [documents[0].toJSON()];

    findDuplicateDocumentsByIdMock.returns(duplicateDocuments);

    const result = await validateDocumentsSTStructure(rawStateTransition);

    expectValidationError(result, DuplicateDocumentsError);

    const [error] = result.getErrors();

    expect(error.getDuplicatedDocuments()).to.deep.equal(duplicateDocuments);

    expect(fetchAndValidateDataContractMock).to.be.calledOnceWith(documents[0].toJSON());
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
    expect(validateStateTransitionSignatureMock).to.be.not.called(stateTransition, userId);
    expect(validateIdentityExistenceAndTypeMock).to.be.calledOnceWith(
      userId, [Identity.TYPES.USER, Identity.TYPES.APPLICATION],
    );
  });

  it('should return invalid result if there are duplicate unique index values', async () => {
    const duplicateDocuments = [documents[1].toJSON()];

    findDuplicateDocumentsByIndicesMock.returns(duplicateDocuments);

    const result = await validateDocumentsSTStructure(rawStateTransition);

    expectValidationError(result, DuplicateDocumentsError);

    const [error] = result.getErrors();

    expect(error.getDuplicatedDocuments()).to.deep.equal(duplicateDocuments);

    expect(fetchAndValidateDataContractMock).to.be.calledOnceWith(documents[0].toJSON());
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
    expect(validateStateTransitionSignatureMock).to.be.not.called(stateTransition, userId);
    expect(validateIdentityExistenceAndTypeMock).to.be.calledOnceWith(
      userId, [Identity.TYPES.USER, Identity.TYPES.APPLICATION],
    );
  });

  it('should return invalid result if there are documents with different User IDs', async () => {
    const differentUserId = generateRandomId();

    documents[0].userId = differentUserId;
    rawStateTransition.documents[0].$userId = differentUserId;

    stateTransition = new DocumentsStateTransition(documents);

    const result = await validateDocumentsSTStructure(rawStateTransition);

    expectValidationError(result, STContainsDocumentsFromDifferentUsersError);

    const [error] = result.getErrors();

    expect(error.getRawDocuments()).to.deep.equal([
      documents[0].toJSON(),
      documents[1].toJSON(),
    ]);

    expect(fetchAndValidateDataContractMock).to.be.calledOnceWith(documents[0].toJSON());
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
    expect(validateStateTransitionSignatureMock).to.be.not.called();
    expect(validateIdentityExistenceAndTypeMock).to.be.calledOnceWith(
      differentUserId, [Identity.TYPES.USER, Identity.TYPES.APPLICATION],
    );
  });

  it('should return invalid result with invalid signature', async () => {
    const type = 1;
    const validationError = new InvalidIdentityPublicKeyTypeError(type);

    const validateSignatureResult = new ValidationResult([
      validationError,
    ]);
    validateStateTransitionSignatureMock.resolves(
      validateSignatureResult,
    );

    const result = await validateDocumentsSTStructure(rawStateTransition, dataContract);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.false();

    const [error] = result.getErrors();

    expect(error).to.equal(validationError);

    documents.forEach((document) => {
      expect(validateDocumentMock).to.have.been.calledWith(
        document.toJSON(),
        dataContract,
        { action: document.getAction() },
      );
    });

    expect(findDuplicateDocumentsByIdMock).to.have.been.calledOnceWith(documents);
    expect(findDuplicateDocumentsByIndicesMock).to.be.calledOnceWith(documents, dataContract);
    expect(validateStateTransitionSignatureMock).to.be.calledOnceWith(stateTransition, userId);
    expect(validateIdentityExistenceAndTypeMock).to.be.calledOnceWith(
      userId, [Identity.TYPES.USER, Identity.TYPES.APPLICATION],
    );
  });

  it('should return valid result', async () => {
    const result = await validateDocumentsSTStructure(rawStateTransition, dataContract);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(fetchAndValidateDataContractMock).to.be.calledOnceWith(documents[0].toJSON());
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
    expect(validateStateTransitionSignatureMock).to.be.calledOnceWith(stateTransition, userId);
    expect(validateIdentityExistenceAndTypeMock).to.be.calledOnceWith(
      userId, [Identity.TYPES.USER, Identity.TYPES.APPLICATION],
    );
  });
});
