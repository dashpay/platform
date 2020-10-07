const Ajv = require('ajv');

const Document = require('../../../../../../lib/document/Document');

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
  let validateDocumentsBatchTransitionStructure;
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
      protocolVersion: Document.PROTOCOL_VERSION,
      ownerId,
      contractId: dataContract.getId(),
      transitions: documentTransitions.map((t) => t.toObject()),
      signature: Buffer.alloc(65),
      signaturePublicKeyId: 0,
    }, [dataContract]);

    rawStateTransition = stateTransition.toObject();

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

    validateDocumentsBatchTransitionStructure = validateDocumentsBatchTransitionStructureFactory(
      findDuplicatesByIdMock,
      findDuplicatesByIndicesMock,
      validateStateTransitionSignatureMock,
      validateIdentityExistenceMock,
      stateRepositoryMock,
      validator,
      enrichSpy,
    );
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {
      delete rawStateTransition.protocolVersion;

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('protocolVersion');
    });

    it('should be an integer', async () => {
      rawStateTransition.protocolVersion = '1';

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.protocolVersion');
      expect(error.keyword).to.equal('type');
    });

    it('should not be less than 0', async () => {
      rawStateTransition.protocolVersion = -1;

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('minimum');
      expect(error.dataPath).to.equal('.protocolVersion');
    });

    it('should not be greater than current version (0)', async () => {
      rawStateTransition.protocolVersion = 1;

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('maximum');
      expect(error.dataPath).to.equal('.protocolVersion');
    });
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawStateTransition.type;

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('type');
    });

    it('should be equal 1', async () => {
      rawStateTransition.type = 666;

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.type');
      expect(error.keyword).to.equal('const');
      expect(error.params.allowedValue).to.equal(1);
    });
  });

  describe('ownerId', () => {
    it('should be present', async () => {
      delete rawStateTransition.ownerId;

      const result = await validateDocumentsBatchTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('ownerId');
    });

    it('should be a binary (encoded string)', async () => {
      rawStateTransition.ownerId = 1;

      const result = await validateDocumentsBatchTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.ownerId');
      expect(error.keyword).to.equal('type');
      expect(error.params.type).to.equal('string');
    });

    it('should be no less than 42 chars', async () => {
      rawStateTransition.ownerId = '1'.repeat(41);

      const result = await validateDocumentsBatchTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.ownerId');
      expect(error.keyword).to.equal('minLength');
    });

    it('should be no longer than 44 chars', async () => {
      rawStateTransition.ownerId = '1'.repeat(45);

      const result = await validateDocumentsBatchTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.ownerId');
      expect(error.keyword).to.equal('maxLength');
    });

    it('should be base58 encoded', async () => {
      rawStateTransition.ownerId = '&'.repeat(44);

      const result = await validateDocumentsBatchTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('pattern');
      expect(error.dataPath).to.equal('.ownerId');
    });

    it('should exists', async () => {
      const validationResult = new ValidationResult();
      validationResult.addError(new ConsensusError('no identity'));

      validateIdentityExistenceMock.withArgs(stateTransition.getOwnerId().toBuffer())
        .resolves(validationResult);

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expectValidationError(result);

      const [error] = result.getErrors();

      expect(error.message).to.equal('no identity');

      expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
        dataContract.getId().toBuffer(),
      );

      expect(enrichSpy).to.have.been.calledThrice();

      expect(findDuplicatesByIdMock).to.have.been.calledOnceWithExactly(
        rawStateTransition.transitions,
      );

      expect(findDuplicatesByIndicesMock).to.have.been.calledOnceWithExactly(
        rawStateTransition.transitions, dataContract,
      );

      expect(validateIdentityExistenceMock).to.have.been.calledOnceWithExactly(ownerId.toBuffer());

      expect(validateStateTransitionSignatureMock).to.have.not.been.called();
    });
  });

  describe('transitions', () => {
    it('should be present', async () => {
      delete rawStateTransition.transitions;

      const result = await validateDocumentsBatchTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('transitions');
    });

    it('should be an array', async () => {
      rawStateTransition.transitions = {};

      const result = await validateDocumentsBatchTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.transitions');
      expect(error.keyword).to.equal('type');
    });

    it('should have at least one element', async () => {
      rawStateTransition.transitions = [];

      const result = await validateDocumentsBatchTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.transitions');
      expect(error.keyword).to.equal('minItems');
      expect(error.params.limit).to.equal(1);
    });

    it('should have no more than 10 elements', async () => {
      rawStateTransition.transitions = Array(11).fill({});

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.transitions');
      expect(error.keyword).to.equal('maxItems');
      expect(error.params.limit).to.equal(10);
    });

    it('should have objects as elements', async () => {
      rawStateTransition.transitions = [1];

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.transitions[0]');
      expect(error.keyword).to.equal('type');
    });

    describe('transaction', () => {
      describe('$id', () => {
        it('should be present', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          delete documentTransition.$id;

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('');
          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('$id');
        });

        it('should be a string', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          documentTransition.$id = 1;

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('.$id');
          expect(error.keyword).to.equal('type');
        });

        it('should be no less than 42 chars', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          documentTransition.$id = '1'.repeat(41);

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('.$id');
          expect(error.keyword).to.equal('minLength');
        });

        it('should be no longer than 44 chars', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          documentTransition.$id = '1'.repeat(45);

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('.$id');
          expect(error.keyword).to.equal('maxLength');
        });

        it('should be base58 encoded', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          documentTransition.$id = '&'.repeat(44);

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.keyword).to.equal('pattern');
          expect(error.dataPath).to.equal('.$id');
        });

        it('should no have duplicate IDs in the state transition', async () => {
          const duplicates = [documentTransitions[0].toObject()];

          findDuplicatesByIdMock.returns(duplicates);

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

          expectValidationError(result, DuplicateDocumentTransitionsError);

          const [error] = result.getErrors();

          expect(error.getRawDocumentTransitions()).to.deep.equal(duplicates);

          expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
            dataContract.getId().toBuffer(),
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

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

          expectValidationError(result, MissingDataContractIdError);

          const [error] = result.getErrors();

          expect(error.getRawDocument()).to.equal(firstDocumentTransition);

          expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
            dataContract.getId().toBuffer(),
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

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

          expectValidationError(result, InvalidDataContractIdError);

          const [error] = result.getErrors();

          expect(error.getRawDataContract()).to.equal(firstDocumentTransition.$dataContractId);

          expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
            dataContract.getId().toBuffer(),
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

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

          expectValidationError(result, DataContractNotPresentError);

          const [error] = result.getErrors();

          expect(error.getDataContractId()).to.deep.equal(dataContract.getId());

          expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
            dataContract.getId().toBuffer(),
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

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

          expectValidationError(result, MissingDocumentTypeError);

          const [error] = result.getErrors();

          expect(error.getRawDocument()).to.equal(firstDocumentTransition);

          expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
            dataContract.getId().toBuffer(),
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

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

          expectValidationError(result, InvalidDocumentTypeError);

          const [error] = result.getErrors();

          expect(error.getType()).to.equal(firstDocumentTransition.$type);
          expect(error.getDataContract()).to.equal(dataContract);

          expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
            dataContract.getId().toBuffer(),
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

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

          expectValidationError(result, MissingDocumentTransitionActionError);

          const [error] = result.getErrors();

          expect(error.getRawDocumentTransition()).to.equal(firstDocumentTransition);

          expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
            dataContract.getId().toBuffer(),
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

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

          expectValidationError(result, InvalidDocumentTransitionActionError);

          const [error] = result.getErrors();

          expect(error.getAction()).to.equal(firstDocumentTransition.$action);
          expect(error.getRawDocumentTransition()).to.deep.equal(firstDocumentTransition);

          expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
            dataContract.getId().toBuffer(),
          );

          expect(enrichSpy).to.have.been.calledThrice();
          expect(findDuplicatesByIdMock).to.have.not.been.called();
          expect(findDuplicatesByIndicesMock).to.have.not.been.called();
          expect(validateIdentityExistenceMock).to.have.not.been.called();
          expect(validateStateTransitionSignatureMock).to.have.not.been.called();
        });
      });

      describe('create', () => {
        describe('$id', () => {
          it('should be valid generated ID', async () => {
            const [firstTransition] = rawStateTransition.transitions;

            firstTransition.$id = generateRandomId();

            const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

            expectValidationError(result, InvalidDocumentTransitionIdError);

            const [error] = result.getErrors();

            expect(error.getRawDocumentTransition()).to.deep.equal(firstTransition);

            expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
              dataContract.getId().toBuffer(),
            );

            expect(enrichSpy).to.have.been.calledThrice();

            expect(findDuplicatesByIdMock).to.have.not.been.called();
            expect(findDuplicatesByIndicesMock).to.have.not.been.called();
            expect(validateIdentityExistenceMock).to.have.not.been.called();
            expect(validateStateTransitionSignatureMock).to.have.not.been.called();
          });
        });

        describe('$entropy', () => {
          it('should be present', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            delete documentTransition.$entropy;

            const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.dataPath).to.equal('');
            expect(error.keyword).to.equal('required');
            expect(error.params.missingProperty).to.equal('$entropy');
          });

          it('should be a string', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$entropy = 1;

            const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.dataPath).to.equal('.$entropy');
            expect(error.keyword).to.equal('type');
          });

          it('should be no less than 26 chars', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$entropy = '1'.repeat(24);

            const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.dataPath).to.equal('.$entropy');
            expect(error.keyword).to.equal('minLength');
          });

          it('should be no longer than 35 chars', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$entropy = '1'.repeat(36);

            const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.dataPath).to.equal('.$entropy');
            expect(error.keyword).to.equal('maxLength');
          });

          it('should be valid generated entropy', async () => {
            const [firstTransition] = rawStateTransition.transitions;

            firstTransition.$entropy = Buffer.alloc(32); // invalid generated entropy

            const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

            expect(result.isValid()).to.be.false();

            const [, error] = result.getErrors();

            expect(error).to.be.an.instanceOf(InvalidDocumentTransitionEntropyError);

            expect(error.getRawDocumentTransition()).to.deep.equal(firstTransition);

            expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
              dataContract.getId().toBuffer(),
            );

            expect(enrichSpy).to.have.been.calledThrice();

            expect(findDuplicatesByIdMock).to.have.not.been.called();
            expect(findDuplicatesByIndicesMock).to.have.not.been.called();
            expect(validateIdentityExistenceMock).to.have.not.been.called();
            expect(validateStateTransitionSignatureMock).to.have.not.been.called();
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
            protocolVersion: Document.PROTOCOL_VERSION,
            ownerId,
            contractId: dataContract.getId(),
            transitions: documentTransitions.map((t) => t.toObject()),
            signature: Buffer.alloc(65),
            signaturePublicKeyId: 0,
          }, [dataContract]);

          rawStateTransition = stateTransition.toObject();
        });

        describe('$revision', () => {
          it('should be present', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            delete documentTransition.$revision;

            const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.params.missingProperty).to.equal('$revision');
            expect(error.keyword).to.equal('required');
          });

          it('should be a number', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$revision = '1';

            const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.dataPath).to.equal('.$revision');
            expect(error.keyword).to.equal('type');
          });

          it('should be multiple of 1.0', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$revision = 1.2;

            const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.dataPath).to.equal('.$revision');
            expect(error.keyword).to.equal('type');
          });

          it('should have a minimum value of 1', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$revision = 0;

            const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.dataPath).to.equal('.$revision');
            expect(error.keyword).to.equal('minimum');
          });
        });
      });

      it('should return invalid result if there are duplicate unique index values', async () => {
        const duplicates = [documentTransitions[1].toObject()];

        findDuplicatesByIndicesMock.returns(duplicates);

        const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

        expectValidationError(result, DuplicateDocumentTransitionsError);

        const [error] = result.getErrors();

        expect(error.getRawDocumentTransitions()).to.deep.equal(duplicates);

        expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
          dataContract.getId().toBuffer(),
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
  });

  describe('signature', () => {
    it('should be present', async () => {
      delete rawStateTransition.signature;

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('signature');
    });

    it('should be a binary (encoded string)', async () => {
      rawStateTransition.signature = 1;

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.signature');
      expect(error.keyword).to.equal('type');
      expect(error.params.type).to.equal('string');
    });

    it('should have length of 65 bytes (87 chars)', async () => {
      rawStateTransition.signature = Buffer.alloc(10);

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.signature');
      expect(error.keyword).to.equal('minLength');
      expect(error.params.limit).to.equal(87);
    });

    it('should be base64 encoded', async () => {
      rawStateTransition.signature = '&'.repeat(87);

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.signature');
      expect(error.keyword).to.equal('pattern');
    });

    it('should be valid', async () => {
      const type = 1;
      const validationError = new InvalidIdentityPublicKeyTypeError(type);

      const validateSignatureResult = new ValidationResult([
        validationError,
      ]);
      validateStateTransitionSignatureMock.resolves(
        validateSignatureResult,
      );

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();

      const [error] = result.getErrors();

      expect(error).to.equal(validationError);

      expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
        dataContract.getId().toBuffer(),
      );

      expect(enrichSpy).to.have.been.calledThrice();

      expect(findDuplicatesByIdMock).to.have.been.calledOnceWithExactly(
        rawStateTransition.transitions,
      );

      expect(findDuplicatesByIndicesMock).to.have.been.calledOnceWithExactly(
        rawStateTransition.transitions, dataContract,
      );

      expect(validateIdentityExistenceMock).to.have.been.calledOnceWithExactly(ownerId.toBuffer());

      expect(validateStateTransitionSignatureMock).to.be.calledOnce();
      expect(validateStateTransitionSignatureMock.getCall(0).args[0]).to.deep.equal(
        stateTransition,
      );
      expect(validateStateTransitionSignatureMock.getCall(0).args[1]).to.deep.equal(
        ownerId,
      );
    });
  });

  describe('signaturePublicKeyId', () => {
    it('should be an integer', async () => {
      rawStateTransition.signaturePublicKeyId = 1.4;

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.signaturePublicKeyId');
      expect(error.keyword).to.equal('type');
    });

    it('should not be < 0', async () => {
      rawStateTransition.signaturePublicKeyId = -1;

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.signaturePublicKeyId');
      expect(error.keyword).to.equal('minimum');
    });
  });

  it('should return valid result', async () => {
    const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContract.getId().toBuffer(),
    );

    expect(enrichSpy).to.have.been.calledThrice();

    expect(findDuplicatesByIdMock).to.have.been.calledOnceWithExactly(
      rawStateTransition.transitions,
    );

    expect(findDuplicatesByIndicesMock).to.have.been.calledOnceWithExactly(
      rawStateTransition.transitions, dataContract,
    );

    expect(validateIdentityExistenceMock).to.have.been.calledOnceWithExactly(ownerId.toBuffer());

    expect(validateStateTransitionSignatureMock).to.be.calledOnce();
    expect(validateStateTransitionSignatureMock.getCall(0).args[0]).to.deep.equal(
      stateTransition,
    );
    expect(validateStateTransitionSignatureMock.getCall(0).args[1]).to.deep.equal(
      ownerId,
    );
  });
});
