const { default: getRE2Class } = require('@dashevo/re2-wasm');

const createAjv = require('../../../../../../../lib/ajv/createAjv');

const protocolVersion = require('../../../../../../../lib/version/protocolVersion');

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

const InvalidDocumentTransitionIdError = require('../../../../../../../lib/errors/consensus/basic/document/InvalidDocumentTransitionIdError');
const DataContractNotPresentError = require('../../../../../../../lib/errors/consensus/basic/document/DataContractNotPresentError');
const MissingDataContractIdError = require('../../../../../../../lib/errors/consensus/basic/document/MissingDataContractIdError');
const MissingDocumentTransitionTypeError = require('../../../../../../../lib/errors/consensus/basic/document/MissingDocumentTransitionTypeError');
const InvalidDocumentTypeError = require('../../../../../../../lib/errors/consensus/basic/document/InvalidDocumentTypeError');
const MissingDocumentTransitionActionError = require('../../../../../../../lib/errors/consensus/basic/document/MissingDocumentTransitionActionError');
const InvalidDocumentTransitionActionError = require('../../../../../../../lib/errors/consensus/basic/document/InvalidDocumentTransitionActionError');
const InvalidIdentifierError = require('../../../../../../../lib/errors/consensus/basic/InvalidIdentifierError');
const DuplicateDocumentTransitionsWithIndicesError = require('../../../../../../../lib/errors/consensus/basic/document/DuplicateDocumentTransitionsWithIndicesError');
const DuplicateDocumentTransitionsWithIdsError = require('../../../../../../../lib/errors/consensus/basic/document/DuplicateDocumentTransitionsWithIdsError');
const SomeConsensusError = require('../../../../../../../lib/test/mocks/SomeConsensusError');

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
  let validateProtocolVersionMock;

  beforeEach(async function beforeEach() {
    dataContract = getDataContractFixture();
    documents = getDocumentsFixture(dataContract);

    ownerId = getDocumentsFixture.ownerId;

    documentTransitions = getDocumentTransitionsFixture({
      create: documents,
    });

    stateTransition = new DocumentsBatchTransition({
      protocolVersion: protocolVersion.latestVersion,
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

    validateProtocolVersionMock = this.sinonSandbox.stub().returns(new ValidationResult());

    validateDocumentsBatchTransitionBasic = validateDocumentsBatchTransitionBasicFactory(
      findDuplicatesByIdMock,
      findDuplicatesByIndicesMock,
      stateRepositoryMock,
      validator,
      enrichSpy,
      validatePartialCompoundIndicesMock,
      validateProtocolVersionMock,
    );
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {
      delete rawStateTransition.protocolVersion;

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('protocolVersion');
    });

    it('should be an integer', async () => {
      rawStateTransition.protocolVersion = '1';

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/protocolVersion');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be valid', async () => {
      rawStateTransition.protocolVersion = -1;

      const protocolVersionError = new SomeConsensusError('test');
      const protocolVersionResult = new ValidationResult([
        protocolVersionError,
      ]);

      validateProtocolVersionMock.returns(protocolVersionResult);

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectValidationError(result, SomeConsensusError);

      const [error] = result.getErrors();

      expect(error).to.equal(protocolVersionError);

      expect(validateProtocolVersionMock).to.be.calledOnceWith(
        rawStateTransition.protocolVersion,
      );
    });
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawStateTransition.type;

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('type');
    });

    it('should be equal 1', async () => {
      rawStateTransition.type = 666;

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/type');
      expect(error.getKeyword()).to.equal('const');
      expect(error.getParams().allowedValue).to.equal(1);
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

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('ownerId');
    });

    it('should be a byte array', async () => {
      rawStateTransition.ownerId = new Array(32).fill('string');

      const result = await validateDocumentsBatchTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/ownerId/0');
      expect(error.getKeyword()).to.equal('type');

      expect(byteArrayError.getKeyword()).to.equal('byteArray');
    });

    it('should be no less than 32 bytes', async () => {
      rawStateTransition.ownerId = Buffer.alloc(31);

      const result = await validateDocumentsBatchTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/ownerId');
      expect(error.getKeyword()).to.equal('minItems');
    });

    it('should be no longer than 32 bytes', async () => {
      rawStateTransition.ownerId = Buffer.alloc(33);

      const result = await validateDocumentsBatchTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/ownerId');
      expect(error.getKeyword()).to.equal('maxItems');
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
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('transitions');
    });

    it('should be an array', async () => {
      rawStateTransition.transitions = {};

      const result = await validateDocumentsBatchTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/transitions');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should have at least one element', async () => {
      rawStateTransition.transitions = [];

      const result = await validateDocumentsBatchTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/transitions');
      expect(error.getKeyword()).to.equal('minItems');
      expect(error.getParams().limit).to.equal(1);
    });

    it('should have no more than 10 elements', async () => {
      rawStateTransition.transitions = Array(11).fill({});

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/transitions');
      expect(error.getKeyword()).to.equal('maxItems');
      expect(error.getParams().limit).to.equal(10);
    });

    it('should have objects as elements', async () => {
      rawStateTransition.transitions = [1];

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/transitions/0');
      expect(error.getKeyword()).to.equal('type');
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
          expect(error.getKeyword()).to.equal('required');
          expect(error.getParams().missingProperty).to.equal('$id');
        });

        it('should be a byte array', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          documentTransition.$id = new Array(32).fill('string');

          const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

          expectJsonSchemaError(result, 2);

          const [error, byteArrayError] = result.getErrors();

          expect(error.instancePath).to.equal('/$id/0');
          expect(error.getKeyword()).to.equal('type');

          expect(byteArrayError.getKeyword()).to.equal('byteArray');
        });

        it('should be no less than 32 bytes', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          documentTransition.$id = Buffer.alloc(31);

          const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.instancePath).to.equal('/$id');
          expect(error.getKeyword()).to.equal('minItems');
          expect(error.getParams().limit).to.equal(32);
        });

        it('should be no longer than 32 bytes', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          documentTransition.$id = Buffer.alloc(33);

          const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.instancePath).to.equal('/$id');
          expect(error.getKeyword()).to.equal('maxItems');
          expect(error.getParams().limit).to.equal(32);
        });

        it('should no have duplicate IDs in the state transition', async () => {
          const duplicates = [documentTransitions[0].toObject()];

          findDuplicatesByIdMock.returns(duplicates);

          const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

          expectValidationError(result, DuplicateDocumentTransitionsWithIdsError);

          const [error] = result.getErrors();

          expect(error.getCode()).to.equal(1019);
          expect(error.getDocumentTransitionReferences()).to.deep.equal(
            duplicates.map((d) => [d.$type, d.$id]),
          );

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

          expect(error.getCode()).to.equal(1025);

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

          expect(error.getCode()).to.equal(1006);

          expect(error.getIdentifierName()).to.equal('$dataContractId');

          expect(error.getIdentifierError()).to.be.instanceOf(Error);
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

          expect(error.getCode()).to.equal(1018);
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

          expectValidationError(result, MissingDocumentTransitionTypeError);

          const [error] = result.getErrors();

          expect(error.getCode()).to.equal(1027);

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

          expect(error.getCode()).to.equal(1024);
          expect(error.getType()).to.equal(firstDocumentTransition.$type);

          expect(Buffer.isBuffer(error.getDataContractId())).to.be.true();
          expect(error.getDataContractId()).to.deep.equal(dataContract.getId().toBuffer());

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

          expect(error.getCode()).to.equal(1026);

          expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
            dataContract.getId(),
          );

          expect(enrichSpy).to.have.been.calledThrice();
          expect(findDuplicatesByIdMock).to.have.not.been.called();
          expect(findDuplicatesByIndicesMock).to.have.not.been.called();
        });

        it('should throw InvalidDocumentTransitionActionError if action is not valid', async () => {
          const [firstDocumentTransition] = rawStateTransition.transitions;

          firstDocumentTransition.$action = 4;

          try {
            await validateDocumentsBatchTransitionBasic(rawStateTransition);
          } catch (e) {
            expect(e).to.be.instanceOf(InvalidDocumentTransitionActionError);
            expect(e.getAction()).to.equal(firstDocumentTransition.$action);
            expect(e.getRawDocumentTransition()).to.deep.equal(firstDocumentTransition);

            expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnceWithExactly(
              dataContract.getId(),
            );

            expect(enrichSpy).to.have.been.calledThrice();
            expect(findDuplicatesByIdMock).to.have.not.been.called();
            expect(findDuplicatesByIndicesMock).to.have.not.been.called();
          }
        });
      });

      describe('create', () => {
        describe('$id', () => {
          it('should be valid generated ID', async () => {
            const [firstTransition] = rawStateTransition.transitions;

            const expectedId = firstTransition.$id;
            firstTransition.$id = generateRandomIdentifier();

            const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

            expectValidationError(result, InvalidDocumentTransitionIdError);

            const [error] = result.getErrors();

            expect(error.getCode()).to.equal(1023);

            expect(Buffer.isBuffer(error.getExpectedId())).to.be.true();
            expect(error.getExpectedId()).to.deep.equal(expectedId);

            expect(Buffer.isBuffer(error.getInvalidId())).to.be.true();
            expect(error.getInvalidId()).to.deep.equal(firstTransition.$id);

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
            expect(error.getKeyword()).to.equal('required');
            expect(error.getParams().missingProperty).to.equal('$entropy');
          });

          it('should be a byte array', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$entropy = new Array(32).fill('string');

            const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

            expectJsonSchemaError(result, 2);

            const [error, byteArrayError] = result.getErrors();

            expect(error.instancePath).to.equal('/$entropy/0');
            expect(error.getKeyword()).to.equal('type');

            expect(byteArrayError.getKeyword()).to.equal('byteArray');
          });

          it('should be no less than 32 bytes', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$entropy = Buffer.alloc(31);

            const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.instancePath).to.equal('/$entropy');
            expect(error.getKeyword()).to.equal('minItems');
            expect(error.getParams().limit).to.equal(32);
          });

          it('should be no longer than 32 bytes', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$entropy = Buffer.alloc(33);

            const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.instancePath).to.equal('/$entropy');
            expect(error.getKeyword()).to.equal('maxItems');
            expect(error.getParams().limit).to.equal(32);
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
            protocolVersion: protocolVersion.latestVersion,
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

            expect(error.getParams().missingProperty).to.equal('$revision');
            expect(error.getKeyword()).to.equal('required');
          });

          it('should be a number', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$revision = '1';

            const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.instancePath).to.equal('/$revision');
            expect(error.getKeyword()).to.equal('type');
          });

          it('should be multiple of 1.0', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$revision = 1.2;

            const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.instancePath).to.equal('/$revision');
            expect(error.getKeyword()).to.equal('type');
          });

          it('should have a minimum value of 1', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$revision = 0;

            const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.instancePath).to.equal('/$revision');
            expect(error.getKeyword()).to.equal('minimum');
          });
        });
      });

      describe('delete', () => {
        beforeEach(() => {
          documentTransitions = getDocumentTransitionsFixture({
            create: [],
            replace: [],
            delete: documents,
          });

          stateTransition = new DocumentsBatchTransition({
            protocolVersion: protocolVersion.latestVersion,
            ownerId,
            contractId: dataContract.getId(),
            transitions: documentTransitions.map((t) => t.toObject()),
            signature: Buffer.alloc(65),
            signaturePublicKeyId: 0,
          }, [dataContract]);

          rawStateTransition = stateTransition.toObject();
        });

        it('should return invalid result if delete transaction is not valid', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          delete documentTransition.$id;

          const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.getParams().missingProperty).to.equal('$id');
          expect(error.getKeyword()).to.equal('required');
        });
      });

      it('should return invalid result if there are duplicate unique index values', async () => {
        const duplicates = [documentTransitions[1].toObject()];

        findDuplicatesByIndicesMock.returns(duplicates);

        const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

        expectValidationError(result, DuplicateDocumentTransitionsWithIndicesError);

        const [error] = result.getErrors();

        expect(error.getCode()).to.equal(1020);

        expect(error.getDocumentTransitionReferences()).to.deep.equal(
          duplicates.map((d) => [d.$type, d.$id]),
        );

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
        const consensusError = new SomeConsensusError('error');

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
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('signature');
    });

    it('should be a byte array', async () => {
      rawStateTransition.signature = new Array(65).fill('string');

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.instancePath).to.equal('/signature/0');
      expect(error.getKeyword()).to.equal('type');

      expect(byteArrayError.getKeyword()).to.equal('byteArray');
    });

    it('should be not less than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(64);

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/signature');
      expect(error.getKeyword()).to.equal('minItems');
      expect(error.getParams().limit).to.equal(65);
    });

    it('should be not longer than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(66);

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/signature');
      expect(error.getKeyword()).to.equal('maxItems');
      expect(error.getParams().limit).to.equal(65);
    });
  });

  describe('signaturePublicKeyId', () => {
    it('should be an integer', async () => {
      rawStateTransition.signaturePublicKeyId = 1.4;

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/signaturePublicKeyId');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should not be < 0', async () => {
      rawStateTransition.signaturePublicKeyId = -1;

      const result = await validateDocumentsBatchTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/signaturePublicKeyId');
      expect(error.getKeyword()).to.equal('minimum');
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
