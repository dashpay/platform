const { default: getRE2Class } = require('@dashevo/re2-wasm');

const createAjv = require('../../../../../../../lib/ajv/createAjv');

const { protocolVersion } = require('../../../../../../../lib/protocolVersion');

const JsonSchemaValidator = require('../../../../../../../lib/validation/JsonSchemaValidator');

const generateRandomIdentifier = require('../../../../../../../lib/test/utils/generateRandomIdentifier');

const enrichDataContractWithBaseSchema = require('../../../../../../../lib/dataContract/enrichDataContractWithBaseSchema');

const DocumentsBatchTransition = require('../../../../../../../lib/document/stateTransition/DocumentsBatchTransition/DocumentsBatchTransition');

const getDocumentTransitionsFixture = require('../../../../../../../lib/test/fixtures/getDocumentTransitionsFixture');
const getDocumentsFixture = require('../../../../../../../lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('../../../../../../../lib/test/fixtures/getDataContractFixture');

const ValidationResult = require('../../../../../../../lib/validation/ValidationResult');

const validateDocumentsBatchTransitionBasicFactory = require('../../../../../../../lib/document/stateTransition/DocumentsBatchTransition/validation/basic/validateDocumentsBatchTransitionBasicFactory');

const { expectValidationError, expectJsonSchemaError } = require('../../../../../../../lib/test/expect/expectError');

const createStateRepositoryMock = require('../../../../../../../lib/test/mocks/createStateRepositoryMock');

const ConsensusError = require('../../../../../../../lib/errors/ConsensusError');
const DuplicateDocumentTransitionsError = require('../../../../../../../lib/errors/DuplicateDocumentTransitionsError');
const InvalidDocumentTransitionIdError = require('../../../../../../../lib/errors/InvalidDocumentTransitionIdError');
const DataContractNotPresentError = require('../../../../../../../lib/errors/DataContractNotPresentError');
const MissingDataContractIdError = require('../../../../../../../lib/errors/MissingDataContractIdError');
const MissingDocumentTypeError = require('../../../../../../../lib/errors/MissingDocumentTypeError');
const InvalidDocumentTypeError = require('../../../../../../../lib/errors/InvalidDocumentTypeError');
const MissingDocumentTransitionActionError = require('../../../../../../../lib/errors/MissingDocumentTransitionActionError');
const InvalidDocumentTransitionActionError = require('../../../../../../../lib/errors/InvalidDocumentTransitionActionError');
const InvalidIdentifierError = require('../../../../../../../lib/errors/InvalidIdentifierError');
const IdentifierError = require('../../../../../../../lib/identifier/errors/IdentifierError');

describe('validateDocumentsBatchTransitionBasicFactory', () => {
  let dataContract;
  let documents;
  let rawStateTransition;
  let findDuplicatesByIdMock;
  let findDuplicatesByIndicesMock;
  let validateDocumentsBatchTransitionBasic;
  let stateTransition;
  let ownerId;
  let stateRepositoryMock;
  let validator;
  let enrichSpy;
  let documentTransitions;
  let validatePartialCompoundIndicesMock;

  beforeEach(async function beforeEach() {
    dataContract = getDataContractFixture();
    documents = getDocumentsFixture(dataContract);

    ownerId = getDocumentsFixture.ownerId;

    documentTransitions = getDocumentTransitionsFixture({
      create: documents,
    });

    stateTransition = new DocumentsBatchTransition({
      protocolVersion,
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

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchDataContract.resolves(dataContract);

    const RE2 = await getRE2Class();
    const ajv = createAjv(RE2);

    validator = new JsonSchemaValidator(ajv);

    enrichSpy = this.sinonSandbox.spy(enrichDataContractWithBaseSchema);

    validatePartialCompoundIndicesMock = this.sinonSandbox.stub().returns(
      new ValidationResult(),
    );

    validateDocumentsBatchTransitionBasic = validateDocumentsBatchTransitionBasicFactory(
      findDuplicatesByIdMock,
      findDuplicatesByIndicesMock,
      stateRepositoryMock,
      validator,
      enrichSpy,
      validatePartialCompoundIndicesMock,
    );
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {
      delete rawStateTransition.protocolVersion;

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('protocolVersion');
    });

    it('should be an integer', async () => {
      rawStateTransition.protocolVersion = '1';

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/protocolVersion');
      expect(error.keyword).to.equal('type');
    });

    it('should not be less than 0', async () => {
      rawStateTransition.protocolVersion = -1;

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('minimum');
      expect(error.instancePath).to.equal('/protocolVersion');
    });

    it('should not be greater than current version (0)', async () => {
      rawStateTransition.protocolVersion = 1;

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('maximum');
      expect(error.instancePath).to.equal('/protocolVersion');
    });
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawStateTransition.type;

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('type');
    });

    it('should be equal 1', async () => {
      rawStateTransition.type = 666;

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/type');
      expect(error.keyword).to.equal('const');
      expect(error.params.allowedValue).to.equal(1);
    });
  });

  describe('ownerId', () => {
    it('should be present', async () => {
      delete rawStateTransition.ownerId;

      const result = await validateDocumentsBatchTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('ownerId');
    });

    it('should be a byte array', async () => {
      rawStateTransition.ownerId = new Array(32).fill('string');

      const result = await validateDocumentsBatchTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.instancePath).to.equal('/ownerId/0');
      expect(error.keyword).to.equal('type');

      expect(byteArrayError.keyword).to.equal('byteArray');
    });

    it('should be no less than 32 bytes', async () => {
      rawStateTransition.ownerId = Buffer.alloc(31);

      const result = await validateDocumentsBatchTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/ownerId');
      expect(error.keyword).to.equal('minItems');
    });

    it('should be no longer than 32 bytes', async () => {
      rawStateTransition.ownerId = Buffer.alloc(33);

      const result = await validateDocumentsBatchTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/ownerId');
      expect(error.keyword).to.equal('maxItems');
    });
  });

  describe('document transitions', () => {
    it('should be present', async () => {
      delete rawStateTransition.transitions;

      const result = await validateDocumentsBatchTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('transitions');
    });

    it('should be an array', async () => {
      rawStateTransition.transitions = {};

      const result = await validateDocumentsBatchTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/transitions');
      expect(error.keyword).to.equal('type');
    });

    it('should have at least one element', async () => {
      rawStateTransition.transitions = [];

      const result = await validateDocumentsBatchTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/transitions');
      expect(error.keyword).to.equal('minItems');
      expect(error.params.limit).to.equal(1);
    });

    it('should have no more than 10 elements', async () => {
      rawStateTransition.transitions = Array(11).fill({});

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/transitions');
      expect(error.keyword).to.equal('maxItems');
      expect(error.params.limit).to.equal(10);
    });

    it('should have objects as elements', async () => {
      rawStateTransition.transitions = [1];

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/transitions/0');
      expect(error.keyword).to.equal('type');
    });

    describe('document transition', () => {
      describe('$id', () => {
        it('should be present', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          delete documentTransition.$id;

          const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.instancePath).to.equal('');
          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('$id');
        });

        it('should be a byte array', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          documentTransition.$id = new Array(32).fill('string');

          const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

          expectJsonSchemaError(result, 2);

          const [error, byteArrayError] = result.getErrors();

          expect(error.instancePath).to.equal('/$id/0');
          expect(error.keyword).to.equal('type');

          expect(byteArrayError.keyword).to.equal('byteArray');
        });

        it('should be no less than 32 bytes', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          documentTransition.$id = Buffer.alloc(31);

          const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.instancePath).to.equal('/$id');
          expect(error.keyword).to.equal('minItems');
          expect(error.params.limit).to.equal(32);
        });

        it('should be no longer than 32 bytes', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          documentTransition.$id = Buffer.alloc(33);

          const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.instancePath).to.equal('/$id');
          expect(error.keyword).to.equal('maxItems');
          expect(error.params.limit).to.equal(32);
        });

        it('should no have duplicate IDs in the state transition', async () => {
          const duplicates = [documentTransitions[0].toObject()];

          findDuplicatesByIdMock.returns(duplicates);

          const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

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
        });
      });

      describe('$dataContractId', () => {
        it('should be present', async () => {
          const [firstDocumentTransition] = rawStateTransition.transitions;

          delete firstDocumentTransition.$dataContractId;

          const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

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
        });

        it('should be a byte array', async () => {
          const [firstDocumentTransition] = rawStateTransition.transitions;

          firstDocumentTransition.$dataContractId = 'something';

          const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

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
        });

        it('should exists in the state', async () => {
          stateRepositoryMock.fetchDataContract.resolves(undefined);

          const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

          expectValidationError(result, DataContractNotPresentError);

          const [error] = result.getErrors();

          expect(error.getDataContractId()).to.deep.equal(dataContract.getId());

          expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
            dataContract.getId(),
          );

          expect(enrichSpy).to.have.not.been.called();
          expect(findDuplicatesByIdMock).to.have.not.been.called();
          expect(findDuplicatesByIndicesMock).to.have.not.been.called();
        });
      });

      describe('$type', () => {
        it('should be present', async () => {
          const [firstDocumentTransition] = rawStateTransition.transitions;

          delete firstDocumentTransition.$type;

          const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

          expectValidationError(result, MissingDocumentTypeError);

          const [error] = result.getErrors();

          expect(error.getRawDocument()).to.equal(firstDocumentTransition);

          expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
            dataContract.getId(),
          );

          expect(enrichSpy).to.have.been.calledThrice();
          expect(findDuplicatesByIdMock).to.have.not.been.called();
          expect(findDuplicatesByIndicesMock).to.have.not.been.called();
        });

        it('should be defined in Data Contract', async () => {
          const [firstDocumentTransition] = rawStateTransition.transitions;

          firstDocumentTransition.$type = 'wrong';

          const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

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
        });
      });

      describe('$action', () => {
        it('should be present', async () => {
          const [firstDocumentTransition] = rawStateTransition.transitions;

          delete firstDocumentTransition.$action;

          const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

          expectValidationError(result, MissingDocumentTransitionActionError);

          const [error] = result.getErrors();

          expect(error.getRawDocumentTransition()).to.equal(firstDocumentTransition);

          expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
            dataContract.getId(),
          );

          expect(enrichSpy).to.have.been.calledThrice();
          expect(findDuplicatesByIdMock).to.have.not.been.called();
          expect(findDuplicatesByIndicesMock).to.have.not.been.called();
        });

        it('should be create, replace or delete', async () => {
          const [firstDocumentTransition] = rawStateTransition.transitions;

          firstDocumentTransition.$action = 4;

          const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

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
        });
      });

      describe('create', () => {
        describe('$id', () => {
          it('should be valid generated ID', async () => {
            const [firstTransition] = rawStateTransition.transitions;

            firstTransition.$id = generateRandomIdentifier();

            const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

            expectValidationError(result, InvalidDocumentTransitionIdError);

            const [error] = result.getErrors();

            expect(error.getRawDocumentTransition()).to.deep.equal(firstTransition);

            expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
              dataContract.getId(),
            );

            expect(enrichSpy).to.have.been.calledThrice();

            expect(findDuplicatesByIdMock).to.have.not.been.called();
            expect(findDuplicatesByIndicesMock).to.have.not.been.called();
          });
        });

        describe('$entropy', () => {
          it('should be present', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            delete documentTransition.$entropy;

            const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.instancePath).to.equal('');
            expect(error.keyword).to.equal('required');
            expect(error.params.missingProperty).to.equal('$entropy');
          });

          it('should be a byte array', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$entropy = new Array(32).fill('string');

            const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

            expectJsonSchemaError(result, 2);

            const [error, byteArrayError] = result.getErrors();

            expect(error.instancePath).to.equal('/$entropy/0');
            expect(error.keyword).to.equal('type');

            expect(byteArrayError.keyword).to.equal('byteArray');
          });

          it('should be no less than 32 bytes', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$entropy = Buffer.alloc(31);

            const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.instancePath).to.equal('/$entropy');
            expect(error.keyword).to.equal('minItems');
            expect(error.params.limit).to.equal(32);
          });

          it('should be no longer than 32 bytes', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$entropy = Buffer.alloc(33);

            const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.instancePath).to.equal('/$entropy');
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
            protocolVersion,
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

            const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.params.missingProperty).to.equal('$revision');
            expect(error.keyword).to.equal('required');
          });

          it('should be a number', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$revision = '1';

            const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.instancePath).to.equal('/$revision');
            expect(error.keyword).to.equal('type');
          });

          it('should be multiple of 1.0', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$revision = 1.2;

            const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.instancePath).to.equal('/$revision');
            expect(error.keyword).to.equal('type');
          });

          it('should have a minimum value of 1', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$revision = 0;

            const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.instancePath).to.equal('/$revision');
            expect(error.keyword).to.equal('minimum');
          });
        });
      });

      it('should return invalid result if there are duplicate unique index values', async () => {
        const duplicates = [documentTransitions[1].toObject()];

        findDuplicatesByIndicesMock.returns(duplicates);

        const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

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
      });

      it('should return invalid result if compound index doesn\'t contain all fields', async () => {
        const consensusError = new ConsensusError('error');

        validatePartialCompoundIndicesMock.returns(
          new ValidationResult([consensusError]),
        );

        const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

        expectValidationError(result);

        const [error] = result.getErrors();

        expect(error).to.equal(consensusError);

        expect(validatePartialCompoundIndicesMock).to.be.calledOnceWithExactly(
          ownerId.toBuffer(),
          rawStateTransition.transitions,
          dataContract,
        );

        expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
          dataContract.getId(),
        );
        expect(enrichSpy).to.have.been.calledThrice();
      });
    });
  });

  describe('signature', () => {
    it('should be present', async () => {
      delete rawStateTransition.signature;

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('signature');
    });

    it('should be a byte array', async () => {
      rawStateTransition.signature = new Array(65).fill('string');

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.instancePath).to.equal('/signature/0');
      expect(error.keyword).to.equal('type');

      expect(byteArrayError.keyword).to.equal('byteArray');
    });

    it('should be not less than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(64);

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/signature');
      expect(error.keyword).to.equal('minItems');
      expect(error.params.limit).to.equal(65);
    });

    it('should be not longer than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(66);

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/signature');
      expect(error.keyword).to.equal('maxItems');
      expect(error.params.limit).to.equal(65);
    });
  });

  describe('signaturePublicKeyId', () => {
    it('should be an integer', async () => {
      rawStateTransition.signaturePublicKeyId = 1.4;

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/signaturePublicKeyId');
      expect(error.keyword).to.equal('type');
    });

    it('should not be < 0', async () => {
      rawStateTransition.signaturePublicKeyId = -1;

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/signaturePublicKeyId');
      expect(error.keyword).to.equal('minimum');
    });
  });

  it('should return valid result', async () => {
    const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

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
  });
});
