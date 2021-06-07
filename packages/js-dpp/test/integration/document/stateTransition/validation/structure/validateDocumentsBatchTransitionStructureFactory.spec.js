const createAjv = require('../../../../../../lib/ajv/createAjv');

const Document = require('../../../../../../lib/document/Document');

const JsonSchemaValidator = require('../../../../../../lib/validation/JsonSchemaValidator');

const generateRandomIdentifier = require('../../../../../../lib/test/utils/generateRandomIdentifier');

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
const InvalidIdentityPublicKeyTypeError = require('../../../../../../lib/errors/InvalidIdentityPublicKeyTypeError');
const DataContractNotPresentError = require('../../../../../../lib/errors/DataContractNotPresentError');
const MissingDataContractIdError = require('../../../../../../lib/errors/MissingDataContractIdError');
const MissingDocumentTypeError = require('../../../../../../lib/errors/MissingDocumentTypeError');
const InvalidDocumentTypeError = require('../../../../../../lib/errors/InvalidDocumentTypeError');
const MissingDocumentTransitionActionError = require('../../../../../../lib/errors/MissingDocumentTransitionActionError');
const InvalidDocumentTransitionActionError = require('../../../../../../lib/errors/InvalidDocumentTransitionActionError');
const InvalidIdentifierError = require('../../../../../../lib/errors/InvalidIdentifierError');
const IdentifierError = require('../../../../../../lib/identifier/errors/IdentifierError');

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

  beforeEach(async function beforeEach() {
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

    validator = new JsonSchemaValidator(await createAjv());

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

      expect(error.dataPath).to.equal('/protocolVersion');
      expect(error.keyword).to.equal('type');
    });

    it('should not be less than 0', async () => {
      rawStateTransition.protocolVersion = -1;

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('minimum');
      expect(error.dataPath).to.equal('/protocolVersion');
    });

    it('should not be greater than current version (0)', async () => {
      rawStateTransition.protocolVersion = 1;

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('maximum');
      expect(error.dataPath).to.equal('/protocolVersion');
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

      expect(error.dataPath).to.equal('/type');
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

    it('should be a byte array', async () => {
      rawStateTransition.ownerId = new Array(32).fill('string');

      const result = await validateDocumentsBatchTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.dataPath).to.equal('/ownerId/0');
      expect(error.keyword).to.equal('type');

      expect(byteArrayError.keyword).to.equal('byteArray');
    });

    it('should be no less than 32 bytes', async () => {
      rawStateTransition.ownerId = Buffer.alloc(31);

      const result = await validateDocumentsBatchTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/ownerId');
      expect(error.keyword).to.equal('minItems');
    });

    it('should be no longer than 32 bytes', async () => {
      rawStateTransition.ownerId = Buffer.alloc(33);

      const result = await validateDocumentsBatchTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/ownerId');
      expect(error.keyword).to.equal('maxItems');
    });

    it('should exists', async () => {
      const validationResult = new ValidationResult();
      validationResult.addError(new ConsensusError('no identity'));

      validateIdentityExistenceMock.withArgs(stateTransition.getOwnerId())
        .resolves(validationResult);

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

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

      expect(error.dataPath).to.equal('/transitions');
      expect(error.keyword).to.equal('type');
    });

    it('should have at least one element', async () => {
      rawStateTransition.transitions = [];

      const result = await validateDocumentsBatchTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/transitions');
      expect(error.keyword).to.equal('minItems');
      expect(error.params.limit).to.equal(1);
    });

    it('should have no more than 10 elements', async () => {
      rawStateTransition.transitions = Array(11).fill({});

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/transitions');
      expect(error.keyword).to.equal('maxItems');
      expect(error.params.limit).to.equal(10);
    });

    it('should have objects as elements', async () => {
      rawStateTransition.transitions = [1];

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/transitions/0');
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

        it('should be a byte array', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          documentTransition.$id = new Array(32).fill('string');

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

          expectJsonSchemaError(result, 2);

          const [error, byteArrayError] = result.getErrors();

          expect(error.dataPath).to.equal('/$id/0');
          expect(error.keyword).to.equal('type');

          expect(byteArrayError.keyword).to.equal('byteArray');
        });

        it('should be no less than 32 bytes', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          documentTransition.$id = Buffer.alloc(31);

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('/$id');
          expect(error.keyword).to.equal('minItems');
          expect(error.params.limit).to.equal(32);
        });

        it('should be no longer than 32 bytes', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          documentTransition.$id = Buffer.alloc(33);

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('/$id');
          expect(error.keyword).to.equal('maxItems');
          expect(error.params.limit).to.equal(32);
        });

        it('should no have duplicate IDs in the state transition', async () => {
          const duplicates = [documentTransitions[0].toObject()];

          findDuplicatesByIdMock.returns(duplicates);

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

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

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

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

        it('should be a byte array', async () => {
          const [firstDocumentTransition] = rawStateTransition.transitions;

          firstDocumentTransition.$dataContractId = 'something';

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

          expectValidationError(result, InvalidIdentifierError);

          const [error] = result.getErrors();

          expect(error.getIdentifierName()).to.equal('$dataContractId');
          expect(error.getIdentifierError()).to.be.instanceOf(IdentifierError);
          expect(error.getIdentifierError().message).to.equal('Identifier expects Buffer');

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

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

          expectValidationError(result, DataContractNotPresentError);

          const [error] = result.getErrors();

          expect(error.getDataContractId()).to.deep.equal(dataContract.getId());

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

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

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

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

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

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

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

          const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

          expectValidationError(result, InvalidDocumentTransitionActionError);

          const [error] = result.getErrors();

          expect(error.getAction()).to.equal(firstDocumentTransition.$action);
          expect(error.getRawDocumentTransition()).to.deep.equal(firstDocumentTransition);

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

      describe('create', () => {
        describe('$id', () => {
          it('should be valid generated ID', async () => {
            const [firstTransition] = rawStateTransition.transitions;

            firstTransition.$id = generateRandomIdentifier();

            const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

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

          it('should be a byte array', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$entropy = new Array(32).fill('string');

            const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

            expectJsonSchemaError(result, 2);

            const [error, byteArrayError] = result.getErrors();

            expect(error.dataPath).to.equal('/$entropy/0');
            expect(error.keyword).to.equal('type');

            expect(byteArrayError.keyword).to.equal('byteArray');
          });

          it('should be no less than 32 bytes', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$entropy = Buffer.alloc(31);

            const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.dataPath).to.equal('/$entropy');
            expect(error.keyword).to.equal('minItems');
            expect(error.params.limit).to.equal(32);
          });

          it('should be no longer than 32 bytes', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$entropy = Buffer.alloc(33);

            const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.dataPath).to.equal('/$entropy');
            expect(error.keyword).to.equal('maxItems');
            expect(error.params.limit).to.equal(32);
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

            expect(error.dataPath).to.equal('/$revision');
            expect(error.keyword).to.equal('type');
          });

          it('should be multiple of 1.0', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$revision = 1.2;

            const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.dataPath).to.equal('/$revision');
            expect(error.keyword).to.equal('type');
          });

          it('should have a minimum value of 1', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$revision = 0;

            const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.dataPath).to.equal('/$revision');
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

    it('should be a byte array', async () => {
      rawStateTransition.signature = new Array(65).fill('string');

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.dataPath).to.equal('/signature/0');
      expect(error.keyword).to.equal('type');

      expect(byteArrayError.keyword).to.equal('byteArray');
    });

    it('should be not less than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(64);

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/signature');
      expect(error.keyword).to.equal('minItems');
      expect(error.params.limit).to.equal(65);
    });

    it('should be not longer than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(66);

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/signature');
      expect(error.keyword).to.equal('maxItems');
      expect(error.params.limit).to.equal(65);
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

      expect(error.dataPath).to.equal('/signaturePublicKeyId');
      expect(error.keyword).to.equal('type');
    });

    it('should not be < 0', async () => {
      rawStateTransition.signaturePublicKeyId = -1;

      const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/signaturePublicKeyId');
      expect(error.keyword).to.equal('minimum');
    });
  });

  it('should return valid result', async () => {
    const result = await validateDocumentsBatchTransitionStructure(rawStateTransition);

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

    expect(validateStateTransitionSignatureMock).to.be.calledOnce();
    expect(validateStateTransitionSignatureMock.getCall(0).args[0]).to.deep.equal(
      stateTransition,
    );
    expect(validateStateTransitionSignatureMock.getCall(0).args[1]).to.deep.equal(
      ownerId,
    );
  });
});
