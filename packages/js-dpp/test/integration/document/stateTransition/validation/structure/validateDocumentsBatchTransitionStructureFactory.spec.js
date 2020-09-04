const Ajv = require('ajv');

const JsonSchemaValidator = require('../../../../../../lib/validation/JsonSchemaValidator');

const generateRandomId = require('../../../../../../lib/test/utils/generateRandomId');

const enrichDataContractWithBaseSchema = require('../../../../../../lib/dataContract/enrichDataContractWithBaseSchema');

const DocumentsBatchTransition = require('../../../../../../lib/document/stateTransition/DocumentsBatchTransition');

const getDocumentTransitionsFixture = require('../../../../../../lib/test/fixtures/getDocumentTransitionsFixture');
const getDocumentsFixture = require('../../../../../../lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('../../../../../../lib/test/fixtures/getDataContractFixture');

const ValidationResult = require('../../../../../../lib/validation/ValidationResult');

const validateDocumentsBatchTransitionStructureFactory = require('../../../../../../lib/document/stateTransition/validation/structure/validateDocumentsBatchTransitionStructureFactory');

const { expectValidationError, expectJsonSchemaError } = require('../../../../../../lib/test/expect/expectError');

const createStateRepositoryMock = require('../../../../../../lib/test/mocks/createStateRepositoryMock');

const ConsensusError = require('../../../../../../lib/errors/ConsensusError');
const DuplicateDocumentTransitionsError = require('../../../../../../lib/errors/DuplicateDocumentTransitionsError');
const InvalidDocumentTransitionIdError = require('../../../../../../lib/errors/InvalidDocumentTransitionIdError');
const InvalidDocumentTransitionEntropyError = require('../../../../../../lib/errors/InvalidDocumentTransitionEntropyError');
const InvalidIdentityPublicKeyTypeError = require('../../../../../../lib/errors/InvalidIdentityPublicKeyTypeError');
const DataContractNotPresentError = require('../../../../../../lib/errors/DataContractNotPresentError');
const MissingDataContractIdError = require('../../../../../../lib/errors/MissingDataContractIdError');
const MissingDocumentTypeError = require('../../../../../../lib/errors/MissingDocumentTypeError');
const InvalidDocumentTypeError = require('../../../../../../lib/errors/InvalidDocumentTypeError');
const MissingDocumentTransitionActionError = require('../../../../../../lib/errors/MissingDocumentTransitionActionError');
const InvalidDocumentTransitionActionError = require('../../../../../../lib/errors/InvalidDocumentTransitionActionError');
const InvalidDataContractIdError = require('../../../../../../lib/errors/InvalidDataContractIdError');

describe('validateDocumentsBatchTransitionStructureFactory', () => {
  let dataContract;
  let documents;
  let rawStateTransition;
  let findDuplicatesByIdMock;
  let findDuplicatesByIndicesMock;
  let validateStructure;
  let stateTransition;
  let validateStateTransitionSignatureMock;
  let ownerId;
  let validateIdentityExistenceMock;
  let stateRepositoryMock;
  let validator;
  let enrichSpy;
  let documentTransitions;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();
    documents = getDocumentsFixture(dataContract);

    ownerId = getDocumentsFixture.ownerId;

    documentTransitions = getDocumentTransitionsFixture({
      create: documents,
    });

    stateTransition = new DocumentsBatchTransition({
      ownerId,
      contractId: dataContract.getId(),
      transitions: documentTransitions.map((t) => t.toObject()),
    }, [dataContract]);
    rawStateTransition = stateTransition.toJSON();

    findDuplicatesByIdMock = this.sinonSandbox.stub().returns([]);
    findDuplicatesByIndicesMock = this.sinonSandbox.stub().returns([]);

    const dataContractValidationResult = new ValidationResult();
    dataContractValidationResult.setData(dataContract);

    const validateSignatureResult = new ValidationResult();
    validateStateTransitionSignatureMock = this.sinonSandbox.stub().resolves(
      validateSignatureResult,
    );

    validateIdentityExistenceMock = this.sinonSandbox.stub().resolves(
      new ValidationResult(),
    );

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchDataContract.resolves(dataContract);

    validator = new JsonSchemaValidator(new Ajv());

    enrichSpy = this.sinonSandbox.spy(enrichDataContractWithBaseSchema);

    validateStructure = validateDocumentsBatchTransitionStructureFactory(
      findDuplicatesByIdMock,
      findDuplicatesByIndicesMock,
      validateStateTransitionSignatureMock,
      validateIdentityExistenceMock,
      stateRepositoryMock,
      validator,
      enrichSpy,
    );
  });

  describe('document transitions', () => {
    describe('create', () => {
      it('should return invalid result if there are documents with wrong generated $id', async () => {
        const [firstTransition] = rawStateTransition.transitions;

        firstTransition.$id = generateRandomId();

        const result = await validateStructure(rawStateTransition);

        expectValidationError(result, InvalidDocumentTransitionIdError);

        const [error] = result.getErrors();

        expect(error.getRawDocumentTransition()).to.deep.equal(firstTransition);

        expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
          dataContract.getId(),
        );

        expect(enrichSpy).to.have.been.calledThrice();

        expect(findDuplicatesByIdMock).to.have.not.been.called();
        expect(findDuplicatesByIndicesMock).to.have.not.been.called();
        expect(validateIdentityExistenceMock).to.have.not.been.called();
        expect(validateStateTransitionSignatureMock).to.have.not.been.called();
      });

      it('should return invalid result if there are documents with wrong $entropy', async () => {
        const [firstTransition] = rawStateTransition.transitions;

        firstTransition.$entropy = 'yVDZaFPD8c6wJeLR1DVDJEBAAtfezBntVx'; // invalid generated entropy

        const result = await validateStructure(rawStateTransition);

        expect(result.isValid()).to.be.false();

        const [, error] = result.getErrors();

        expect(error).to.be.an.instanceOf(InvalidDocumentTransitionEntropyError);

        expect(error.getRawDocumentTransition()).to.deep.equal(firstTransition);

        expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
          dataContract.getId(),
        );

        expect(enrichSpy).to.have.been.calledThrice();

        expect(findDuplicatesByIdMock).to.have.not.been.called();
        expect(findDuplicatesByIndicesMock).to.have.not.been.called();
        expect(validateIdentityExistenceMock).to.have.not.been.called();
        expect(validateStateTransitionSignatureMock).to.have.not.been.called();
      });

      describe('$entropy', () => {
        it('should be present', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          delete documentTransition.$entropy;

          const result = await validateStructure(rawStateTransition);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('');
          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('$entropy');
        });

        it('should be a string', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          documentTransition.$entropy = 1;

          const result = await validateStructure(rawStateTransition);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('.$entropy');
          expect(error.keyword).to.equal('type');
        });

        it('should be no less than 26 chars', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          documentTransition.$entropy = '1'.repeat(24);

          const result = await validateStructure(rawStateTransition);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('.$entropy');
          expect(error.keyword).to.equal('minLength');
        });

        it('should be no longer than 35 chars', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          documentTransition.$entropy = '1'.repeat(36);

          const result = await validateStructure(rawStateTransition);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('.$entropy');
          expect(error.keyword).to.equal('maxLength');
        });
      });
    });

    describe('replace', () => {
      beforeEach(() => {
        documentTransitions = getDocumentTransitionsFixture({
          create: [],
          replace: documents,
        });

        stateTransition = new DocumentsBatchTransition({
          ownerId,
          contractId: dataContract.getId(),
          transitions: documentTransitions.map((t) => t.toJSON()),
        }, [dataContract]);

        rawStateTransition = stateTransition.toJSON();
      });

      describe('$revision', () => {
        it('should be present', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          delete documentTransition.$revision;

          const result = await validateStructure(rawStateTransition);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.params.missingProperty).to.equal('$revision');
          expect(error.keyword).to.equal('required');
        });

        it('should be a number', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          documentTransition.$revision = '1';

          const result = await validateStructure(rawStateTransition);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('.$revision');
          expect(error.keyword).to.equal('type');
        });

        it('should be multiple of 1.0', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          documentTransition.$revision = 1.2;

          const result = await validateStructure(rawStateTransition);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('.$revision');
          expect(error.keyword).to.equal('type');
        });

        it('should have a minimum value of 1', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          documentTransition.$revision = 0;

          const result = await validateStructure(rawStateTransition);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('.$revision');
          expect(error.keyword).to.equal('minimum');
        });
      });
    });

    describe('$id', () => {
      it('should be present', async () => {
        const [documentTransition] = rawStateTransition.transitions;

        delete documentTransition.$id;

        const result = await validateStructure(rawStateTransition);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('$id');
      });

      it('should be a string', async () => {
        const [documentTransition] = rawStateTransition.transitions;

        documentTransition.$id = 1;

        const result = await validateStructure(rawStateTransition);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$id');
        expect(error.keyword).to.equal('type');
      });

      it('should be no less than 42 chars', async () => {
        const [documentTransition] = rawStateTransition.transitions;

        documentTransition.$id = '1'.repeat(41);

        const result = await validateStructure(rawStateTransition);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$id');
        expect(error.keyword).to.equal('minLength');
      });

      it('should be no longer than 44 chars', async () => {
        const [documentTransition] = rawStateTransition.transitions;

        documentTransition.$id = '1'.repeat(45);

        const result = await validateStructure(rawStateTransition);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$id');
        expect(error.keyword).to.equal('maxLength');
      });

      it('should be base58 encoded', async () => {
        const [documentTransition] = rawStateTransition.transitions;

        documentTransition.$id = '&'.repeat(44);

        const result = await validateStructure(rawStateTransition);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.keyword).to.equal('pattern');
        expect(error.dataPath).to.equal('.$id');
      });

      it('should no have duplicate IDs in the state transition', async () => {
        const duplicates = [documentTransitions[0].toJSON()];

        findDuplicatesByIdMock.returns(duplicates);

        const result = await validateStructure(rawStateTransition);

        expectValidationError(result, DuplicateDocumentTransitionsError);

        const [error] = result.getErrors();

        expect(error.getRawDocumentTransitions()).to.deep.equal(duplicates);

        expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
          dataContract.getId(),
        );
        expect(enrichSpy).to.have.been.calledThrice();
        expect(findDuplicatesByIdMock).to.have.been.calledOnceWithExactly(
          rawStateTransition.transitions,
        );
        expect(findDuplicatesByIndicesMock).to.have.been.calledOnceWithExactly(
          rawStateTransition.transitions, dataContract,
        );
        expect(validateIdentityExistenceMock).to.have.not.been.called();
        expect(validateStateTransitionSignatureMock).to.have.not.been.called();
      });
    });

    describe('$dataContractId', () => {
      it('should be present', async () => {
        const [firstDocumentTransition] = rawStateTransition.transitions;

        delete firstDocumentTransition.$dataContractId;

        const result = await validateStructure(rawStateTransition);

        expectValidationError(result, MissingDataContractIdError);

        const [error] = result.getErrors();

        expect(error.getRawDocument()).to.equal(firstDocumentTransition);

        expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
          dataContract.getId(),
        );

        expect(enrichSpy).to.have.been.calledThrice();

        expect(findDuplicatesByIdMock).to.have.been.calledOnceWithExactly(
          rawStateTransition.transitions.slice(1),
        );
        expect(findDuplicatesByIndicesMock).to.have.been.calledOnceWithExactly(
          rawStateTransition.transitions.slice(1), dataContract,
        );
        expect(validateIdentityExistenceMock).to.have.not.been.called();
        expect(validateStateTransitionSignatureMock).to.have.not.been.called();
      });

      it('should be string', async () => {
        const [firstDocumentTransition] = rawStateTransition.transitions;

        firstDocumentTransition.$dataContractId = null;

        const result = await validateStructure(rawStateTransition);

        expectValidationError(result, InvalidDataContractIdError);

        const [error] = result.getErrors();

        expect(error.getRawDataContract()).to.equal(firstDocumentTransition.$dataContractId);

        expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
          dataContract.getId(),
        );

        expect(enrichSpy).to.have.been.calledThrice();

        expect(findDuplicatesByIdMock).to.have.been.calledOnceWithExactly(
          rawStateTransition.transitions.slice(1),
        );
        expect(findDuplicatesByIndicesMock).to.have.been.calledOnceWithExactly(
          rawStateTransition.transitions.slice(1), dataContract,
        );
        expect(validateIdentityExistenceMock).to.have.not.been.called();
        expect(validateStateTransitionSignatureMock).to.have.not.been.called();
      });

      it('should exists in the state', async () => {
        stateRepositoryMock.fetchDataContract.resolves(undefined);

        const result = await validateStructure(rawStateTransition);

        expectValidationError(result, DataContractNotPresentError);

        const [error] = result.getErrors();

        expect(error.getDataContractId()).to.equal(dataContract.getId());

        expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
          dataContract.getId(),
        );

        expect(enrichSpy).to.have.not.been.called();
        expect(findDuplicatesByIdMock).to.have.not.been.called();
        expect(findDuplicatesByIndicesMock).to.have.not.been.called();
        expect(validateIdentityExistenceMock).to.have.not.been.called();
        expect(validateStateTransitionSignatureMock).to.have.not.been.called();
      });
    });

    describe('$type', () => {
      it('should be present', async () => {
        const [firstDocumentTransition] = rawStateTransition.transitions;

        delete firstDocumentTransition.$type;

        const result = await validateStructure(rawStateTransition);

        expectValidationError(result, MissingDocumentTypeError);

        const [error] = result.getErrors();

        expect(error.getRawDocument()).to.equal(firstDocumentTransition);

        expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
          dataContract.getId(),
        );

        expect(enrichSpy).to.have.been.calledThrice();
        expect(findDuplicatesByIdMock).to.have.not.been.called();
        expect(findDuplicatesByIndicesMock).to.have.not.been.called();
        expect(validateIdentityExistenceMock).to.have.not.been.called();
        expect(validateStateTransitionSignatureMock).to.have.not.been.called();
      });

      it('should be defined in Data Contract', async () => {
        const [firstDocumentTransition] = rawStateTransition.transitions;

        firstDocumentTransition.$type = 'wrong';

        const result = await validateStructure(rawStateTransition);

        expectValidationError(result, InvalidDocumentTypeError);

        const [error] = result.getErrors();

        expect(error.getType()).to.equal(firstDocumentTransition.$type);
        expect(error.getDataContract()).to.equal(dataContract);

        expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
          dataContract.getId(),
        );

        expect(enrichSpy).to.have.been.calledThrice();
        expect(findDuplicatesByIdMock).to.have.not.been.called();
        expect(findDuplicatesByIndicesMock).to.have.not.been.called();
        expect(validateIdentityExistenceMock).to.have.not.been.called();
        expect(validateStateTransitionSignatureMock).to.have.not.been.called();
      });
    });

    describe('$action', () => {
      it('should be present', async () => {
        const [firstDocumentTransition] = rawStateTransition.transitions;

        delete firstDocumentTransition.$action;

        const result = await validateStructure(rawStateTransition);

        expectValidationError(result, MissingDocumentTransitionActionError);

        const [error] = result.getErrors();

        expect(error.getRawDocumentTransition()).to.equal(firstDocumentTransition);

        expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
          dataContract.getId(),
        );

        expect(enrichSpy).to.have.been.calledThrice();
        expect(findDuplicatesByIdMock).to.have.not.been.called();
        expect(findDuplicatesByIndicesMock).to.have.not.been.called();
        expect(validateIdentityExistenceMock).to.have.not.been.called();
        expect(validateStateTransitionSignatureMock).to.have.not.been.called();
      });

      it('should be create, replace or delete', async () => {
        const [firstDocumentTransition] = rawStateTransition.transitions;

        firstDocumentTransition.$action = 4;

        const result = await validateStructure(rawStateTransition);

        expectValidationError(result, InvalidDocumentTransitionActionError);

        const [error] = result.getErrors();

        expect(error.getAction()).to.equal(firstDocumentTransition.$action);
        expect(error.getRawDocumentTransition()).to.equal(firstDocumentTransition);

        expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
          dataContract.getId(),
        );

        expect(enrichSpy).to.have.been.calledThrice();
        expect(findDuplicatesByIdMock).to.have.not.been.called();
        expect(findDuplicatesByIndicesMock).to.have.not.been.called();
        expect(validateIdentityExistenceMock).to.have.not.been.called();
        expect(validateStateTransitionSignatureMock).to.have.not.been.called();
      });
    });

    it('should return invalid result if there are duplicate unique index values', async () => {
      const duplicates = [documentTransitions[1].toJSON()];

      findDuplicatesByIndicesMock.returns(duplicates);

      const result = await validateStructure(rawStateTransition);

      expectValidationError(result, DuplicateDocumentTransitionsError);

      const [error] = result.getErrors();

      expect(error.getRawDocumentTransitions()).to.deep.equal(duplicates);

      expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
        dataContract.getId(),
      );
      expect(enrichSpy).to.have.been.calledThrice();
      expect(findDuplicatesByIdMock).to.have.been.calledOnceWithExactly(
        rawStateTransition.transitions,
      );
      expect(findDuplicatesByIndicesMock).to.have.been.calledOnceWithExactly(
        rawStateTransition.transitions, dataContract,
      );
      expect(validateIdentityExistenceMock).to.have.not.been.called();
      expect(validateStateTransitionSignatureMock).to.have.not.been.called();
    });
  });

  it('should return invalid result if there are no identity found', async () => {
    const validationResult = new ValidationResult();
    validationResult.addError(new ConsensusError('no identity'));

    validateIdentityExistenceMock.withArgs(rawStateTransition.ownerId)
      .resolves(validationResult);

    const result = await validateStructure(rawStateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error.message).to.equal('no identity');

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContract.getId(),
    );
    expect(enrichSpy).to.have.been.calledThrice();
    expect(findDuplicatesByIdMock).to.have.been.calledOnceWithExactly(
      rawStateTransition.transitions,
    );
    expect(findDuplicatesByIndicesMock).to.have.been.calledOnceWithExactly(
      rawStateTransition.transitions, dataContract,
    );
    expect(validateIdentityExistenceMock).to.have.been.calledOnceWithExactly(ownerId);
    expect(validateStateTransitionSignatureMock).to.have.not.been.called();
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

    const result = await validateStructure(rawStateTransition, dataContract);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.false();

    const [error] = result.getErrors();

    expect(error).to.equal(validationError);

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContract.getId(),
    );
    expect(enrichSpy).to.have.been.calledThrice();
    expect(findDuplicatesByIdMock).to.have.been.calledOnceWithExactly(
      rawStateTransition.transitions,
    );
    expect(findDuplicatesByIndicesMock).to.have.been.calledOnceWithExactly(
      rawStateTransition.transitions, dataContract,
    );
    expect(validateIdentityExistenceMock).to.have.been.calledOnceWithExactly(ownerId);
    expect(validateStateTransitionSignatureMock.getCall(0).args[0].toJSON()).to.deep.equal(
      stateTransition.toJSON(),
    );
    expect(validateStateTransitionSignatureMock.getCall(0).args[1]).to.deep.equal(
      ownerId,
    );
  });

  it('should return valid result', async () => {
    const result = await validateStructure(rawStateTransition, dataContract);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContract.getId(),
    );
    expect(enrichSpy).to.have.been.calledThrice();
    expect(findDuplicatesByIdMock).to.have.been.calledOnceWithExactly(
      rawStateTransition.transitions,
    );
    expect(findDuplicatesByIndicesMock).to.have.been.calledOnceWithExactly(
      rawStateTransition.transitions, dataContract,
    );
    expect(validateIdentityExistenceMock).to.have.been.calledOnceWithExactly(ownerId);
    expect(validateStateTransitionSignatureMock.getCall(0).args[0].toJSON()).to.deep.equal(
      stateTransition.toJSON(),
    );
    expect(validateStateTransitionSignatureMock.getCall(0).args[1]).to.deep.equal(
      ownerId,
    );
  });
});
