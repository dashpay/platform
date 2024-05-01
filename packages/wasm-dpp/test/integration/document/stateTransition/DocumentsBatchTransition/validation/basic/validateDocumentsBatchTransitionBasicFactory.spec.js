const getDocumentTransitionsFixture = require('../../../../../../../lib/test/fixtures/getDocumentTransitionsFixture');
const getDocumentsFixture = require('../../../../../../../lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('../../../../../../../lib/test/fixtures/getDataContractFixture');
const createStateRepositoryMock = require('../../../../../../../lib/test/mocks/createStateRepositoryMock');

const { default: loadWasmDpp } = require('../../../../../../..');
const { getLatestProtocolVersion } = require('../../../../../../..');
const { expectJsonSchemaError, expectValidationError, expectValueError } = require('../../../../../../../lib/test/expect/expectError');
const generateRandomIdentifierAsync = require('../../../../../../../lib/test/utils/generateRandomIdentifierAsync');

let DataContract;
let DocumentsBatchTransition;
let StateTransitionExecutionContext;
let validateDocumentsBatchTransitionBasic;
let generateDocumentId;
let MissingDataContractIdError;
let DataContractNotPresentError;
let MissingDocumentTransitionTypeError;
let InvalidDocumentTypeError;
let MissingDocumentTransitionActionError;
let InvalidDocumentTransitionActionError;
let InvalidDocumentTransitionIdError;
let DuplicateDocumentTransitionsWithIndicesError;
let DuplicateDocumentTransitionsWithIdsError;
let ValidationResult;
let ProtocolVersionValidator;
let ValueError;

describe.skip('validateDocumentsBatchTransitionBasicFactory', () => {
  let dataContract;
  let documents;
  let rawStateTransition;
  let stateTransition;
  let ownerId;
  let stateRepositoryMock;
  let documentTransitions;
  let protocolVersionValidator;
  let executionContext;

  beforeEach(async function beforeEach() {
    ({
      ProtocolVersionValidator,
      DataContract,
      StateTransitionExecutionContext,
      DocumentsBatchTransition,
      ValidationResult,
      validateDocumentsBatchTransitionBasic,
      generateDocumentId,
      MissingDataContractIdError,
      DataContractNotPresentError,
      MissingDocumentTransitionTypeError,
      InvalidDocumentTypeError,
      MissingDocumentTransitionActionError,
      InvalidDocumentTransitionActionError,
      InvalidDocumentTransitionIdError,
      DuplicateDocumentTransitionsWithIndicesError,
      DuplicateDocumentTransitionsWithIdsError,
      ValueError,
    } = await loadWasmDpp());

    const dataContractJs = getDataContractFixture();
    dataContract = new DataContract(dataContractJs.toObject());

    documents = getDocumentsFixture(dataContractJs);
    ownerId = getDocumentsFixture.ownerId;

    documentTransitions = getDocumentTransitionsFixture({
      create: documents,
    });

    executionContext = new StateTransitionExecutionContext();

    stateTransition = new DocumentsBatchTransition({
      protocolVersion: getLatestProtocolVersion(),
      ownerId,
      contractId: dataContract.getId(),
      transitions: documentTransitions.map((t) => t.toObject()),
      signature: Buffer.alloc(65),
      signaturePublicKeyId: 0,
    }, [dataContract.clone()]);

    rawStateTransition = stateTransition.toObject();

    stateRepositoryMock = createStateRepositoryMock(this.sinon);
    stateRepositoryMock.fetchDataContract.resolves(dataContract.clone());

    protocolVersionValidator = new ProtocolVersionValidator();
  });

  describe('protocolVersion', () => {
    it('should be present - Rust', async () => {
      delete rawStateTransition.protocolVersion;

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('protocolVersion');
    });

    it('should be an integer - Rust', async () => {
      rawStateTransition.protocolVersion = '1';

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectValueError(result, 1);

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(ValueError);
    });

    it('should be valid - Rust', async () => {
      rawStateTransition.protocolVersion = -1;
      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectValueError(result, 1);

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(ValueError);
      expect(error.getMessage()).equal('integer out of bounds');
    });
  });

  describe('type', () => {
    it('should be present - Rust', async () => {
      delete rawStateTransition.type;

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('type');
    });

    it('should be equal 1 - Rust', async () => {
      rawStateTransition.type = 666;

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/type');
      expect(error.getKeyword()).to.equal('const');
      expect(error.getParams().allowedValue).to.equal(1);
    });
  });

  describe('ownerId', () => {
    it('should be present - Rust', async () => {
      delete rawStateTransition.ownerId;

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('ownerId');
    });

    it('should be a byte array - Rust', async () => {
      rawStateTransition.ownerId = new Array(32).fill('string');

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectValueError(result, 1);

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(ValueError);
    });

    it('should be no less than 32 bytes - Rust', async () => {
      rawStateTransition.ownerId = Buffer.alloc(31);

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectValueError(result, 1);

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(ValueError);
    });

    it('should be no longer than 32 bytes - Rust', async () => {
      rawStateTransition.ownerId = Buffer.alloc(33);

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectValueError(result, 1);

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(ValueError);
    });
  });

  describe('document transitions', () => {
    it('should be present Rust', async () => {
      delete rawStateTransition.transitions;

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );
      await expectValueError(result, 1);

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(ValueError);
    });

    it('should be an array - Rust', async () => {
      rawStateTransition.transitions = {};

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectValueError(result, 1);

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(ValueError);
    });

    it('should have at least one element - Rust', async () => {
      rawStateTransition.transitions = [];

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/transitions');
      expect(error.getKeyword()).to.equal('minItems');
      expect(error.getParams().minItems).to.equal(1);
    });

    it('should have no more than 10 elements - Rust', async () => {
      rawStateTransition.transitions = Array(11).fill({});

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/transitions');
      expect(error.getKeyword()).to.equal('maxItems');
      expect(error.getParams().maxItems).to.equal(10);
    });

    it('should have objects as elements - Rust', async () => {
      rawStateTransition.transitions = [1];

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectValueError(result, 1);

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(ValueError);
    });

    describe('document transition', () => {
      describe('$id', () => {
        it('should be present - Rust', async () => {
          const [documentTransition] = rawStateTransition.transitions;
          delete documentTransition.$id;

          const result = await validateDocumentsBatchTransitionBasic(
            protocolVersionValidator,
            stateRepositoryMock,
            rawStateTransition,
            executionContext,
          );

          await expectJsonSchemaError(result, 1);

          const [error] = result.getErrors();

          expect(error.getInstancePath()).to.equal('');
          expect(error.getKeyword()).to.equal('required');
          expect(error.getParams().missingProperty).to.equal('$id');
        });

        it('should be a byte array - Rust', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          documentTransition.$id = new Array(32).fill('string');

          const result = await validateDocumentsBatchTransitionBasic(
            protocolVersionValidator,
            stateRepositoryMock,
            rawStateTransition,
            executionContext,
          );

          await expectValueError(result, 1);

          const [error] = result.getErrors();

          expect(error).to.be.an.instanceOf(ValueError);
        });

        it('should be no less than 32 bytes - Rust', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          documentTransition.$id = Buffer.alloc(31);

          const result = await validateDocumentsBatchTransitionBasic(
            protocolVersionValidator,
            stateRepositoryMock,
            rawStateTransition,
            executionContext,
          );

          await expectValueError(result, 1);

          const [error] = result.getErrors();

          expect(error).to.be.an.instanceOf(ValueError);
        });

        it('should be no longer than 32 bytes - Rust', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          documentTransition.$id = Buffer.alloc(33);

          const result = await validateDocumentsBatchTransitionBasic(
            protocolVersionValidator,
            stateRepositoryMock,
            rawStateTransition,
            executionContext,
          );
          await expectValueError(result, 1);

          const [error] = result.getErrors();

          expect(error).to.be.an.instanceOf(ValueError);
        });

        it('should no have duplicate IDs in the state transition - Rust', async () => {
          const [documentTransition] = documentTransitions.map((t) => t.toObject());

          stateTransition = new DocumentsBatchTransition({
            protocolVersion: getLatestProtocolVersion(),
            ownerId,
            contractId: dataContract.getId(),
            transitions: [documentTransition, documentTransition],
            signature: Buffer.alloc(65),
            signaturePublicKeyId: 0,
          }, [dataContract.clone()]);

          rawStateTransition = stateTransition.toObject();

          const result = await validateDocumentsBatchTransitionBasic(
            protocolVersionValidator,
            stateRepositoryMock,
            rawStateTransition,
            executionContext,
          );

          await expectValidationError(result, DuplicateDocumentTransitionsWithIdsError);

          const [error] = result.getErrors();

          expect(error.getCode()).to.equal(1019);

          expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
          const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
          expect(fetchDataContractId.toBuffer()).is.deep.equal(dataContract.getId().toBuffer());
        });
      });

      describe('$dataContractId', () => {
        it('should be present - Rust', async () => {
          const [firstDocumentTransition] = rawStateTransition.transitions;

          delete firstDocumentTransition.$dataContractId;

          const result = await validateDocumentsBatchTransitionBasic(
            protocolVersionValidator,
            stateRepositoryMock,
            rawStateTransition,
            executionContext,
          );

          await expectValidationError(result, MissingDataContractIdError);

          const [error] = result.getErrors();
          expect(error.getCode()).to.equal(1025);

          expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
          const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
          expect(fetchDataContractId.toBuffer()).is.deep.equal(dataContract.getId().toBuffer());
        });

        it('should be a byte array - Rust', async () => {
          const [firstDocumentTransition] = rawStateTransition.transitions;

          firstDocumentTransition.$dataContractId = Buffer.alloc(10);

          const result = await validateDocumentsBatchTransitionBasic(
            protocolVersionValidator,
            stateRepositoryMock,
            rawStateTransition,
            executionContext,
          );

          await expectValueError(result);

          const [error] = result.getErrors();

          expect(error.getMessage()).to.equal('byte length not 32 bytes error: Trying to replace into an identifier, but not 32 bytes long');

          // we won't call fetch data contract, because the state transition structure validation
          // happens first
          expect(stateRepositoryMock.fetchDataContract).to.have.not.been.called();
        });

        it('should exists in the state - Rust', async () => {
          stateRepositoryMock.fetchDataContract.resolves(undefined);

          const result = await validateDocumentsBatchTransitionBasic(
            protocolVersionValidator,
            stateRepositoryMock,
            rawStateTransition,
            executionContext,
          );

          await expectValidationError(result, DataContractNotPresentError);

          const [error] = result.getErrors();

          expect(error.getCode()).to.equal(1018);
          expect(error.getDataContractId()).to.deep.equal(dataContract.getId().toBuffer());

          expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
          const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
          expect(fetchDataContractId.toBuffer()).is.deep.equal(dataContract.getId().toBuffer());
        });
      });

      describe('$type', () => {
        it('should be present - Rust', async () => {
          const [firstDocumentTransition] = rawStateTransition.transitions;

          delete firstDocumentTransition.$type;

          const result = await validateDocumentsBatchTransitionBasic(
            protocolVersionValidator,
            stateRepositoryMock,
            rawStateTransition,
            executionContext,
          );
          await expectValidationError(result, MissingDocumentTransitionTypeError);

          const [error] = result.getErrors();

          expect(error.getCode()).to.equal(1027);

          expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
          const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
          expect(fetchDataContractId.toBuffer()).is.deep.equal(dataContract.getId().toBuffer());
        });

        it('should be defined in Data Contract - Rust', async () => {
          const [firstDocumentTransition] = rawStateTransition.transitions;

          firstDocumentTransition.$type = 'wrong';

          const result = await validateDocumentsBatchTransitionBasic(
            protocolVersionValidator,
            stateRepositoryMock,
            rawStateTransition,
            executionContext,
          );

          await expectValidationError(result, InvalidDocumentTypeError);

          const [error] = result.getErrors();

          expect(error.getCode()).to.equal(1024);
          expect(error.getType()).to.equal(firstDocumentTransition.$type);

          expect(error.getDataContractId()).to.deep.equal(dataContract.getId().toBuffer());

          expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
          const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
          expect(fetchDataContractId.toBuffer()).is.deep.equal(dataContract.getId().toBuffer());
        });
      });

      describe('$action', () => {
        it('should be present - Rust', async () => {
          const [firstDocumentTransition] = rawStateTransition.transitions;

          delete firstDocumentTransition.$action;

          const result = await validateDocumentsBatchTransitionBasic(
            protocolVersionValidator,
            stateRepositoryMock,
            rawStateTransition,
            executionContext,
          );

          await expectValidationError(result, MissingDocumentTransitionActionError);

          const [error] = result.getErrors();

          expect(error.getCode()).to.equal(1026);

          expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
          const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
          expect(fetchDataContractId.toBuffer()).is.deep.equal(dataContract.getId().toBuffer());
        });

        it('should throw InvalidDocumentTransitionActionError if action is not valid - Rust', async () => {
          const [firstDocumentTransition] = rawStateTransition.transitions;

          firstDocumentTransition.$action = 4;

          try {
            await validateDocumentsBatchTransitionBasic(
              protocolVersionValidator,
              stateRepositoryMock,
              rawStateTransition,
              executionContext,
            );
          } catch (e) {
            expect(e).to.be.instanceOf(InvalidDocumentTransitionActionError);
            expect(e.getAction()).to.equal(firstDocumentTransition.$action);
            expect(e.getRawDocumentTransition()).to.deep.equal(firstDocumentTransition);

            expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
            const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
            expect(fetchDataContractId.toBuffer()).is.deep.equal(dataContract.getId().toBuffer());
          }
        });
      });

      describe('create', () => {
        describe('$id', () => {
          it('should be valid generated ID - Rust', async () => {
            const [firstTransition] = rawStateTransition.transitions;

            const expectedId = firstTransition.$id;
            firstTransition.$id = (await generateRandomIdentifierAsync()).toBuffer();

            const result = await validateDocumentsBatchTransitionBasic(
              protocolVersionValidator,
              stateRepositoryMock,
              rawStateTransition,
              executionContext,
            );

            const [error] = result.getErrors();

            await expectValidationError(result, InvalidDocumentTransitionIdError);

            expect(error.getCode()).to.equal(1023);

            expect(error.getExpectedId()).to.deep.equal(expectedId);
            expect(error.getInvalidId()).to.deep.equal(firstTransition.$id);

            expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
            const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
            expect(fetchDataContractId.toBuffer()).is.deep.equal(dataContract.getId().toBuffer());
          });
        });

        describe('$entropy', () => {
          it('should be present - Rust', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            delete documentTransition.$entropy;

            const result = await validateDocumentsBatchTransitionBasic(
              protocolVersionValidator,
              stateRepositoryMock,
              rawStateTransition,
              executionContext,
            );

            await expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.getInstancePath()).to.equal('');
            expect(error.getKeyword()).to.equal('required');
            expect(error.getParams().missingProperty).to.equal('$entropy');
          });

          it('should be a byte array - Rust', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$entropy = new Array(32).fill('string');

            const result = await validateDocumentsBatchTransitionBasic(
              protocolVersionValidator,
              stateRepositoryMock,
              rawStateTransition,
              executionContext,
            );

            await expectJsonSchemaError(result, 32);

            const [error] = result.getErrors();

            expect(error.getInstancePath()).to.equal('/$entropy/0');
            expect(error.getKeyword()).to.equal('type');
          });

          it('should be no less than 32 bytes - Rust', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$entropy = Buffer.alloc(31);

            const result = await validateDocumentsBatchTransitionBasic(
              protocolVersionValidator,
              stateRepositoryMock,
              rawStateTransition,
              executionContext,
            );

            await expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.getInstancePath()).to.equal('/$entropy');
            expect(error.getKeyword()).to.equal('minItems');
            expect(error.getParams().minItems).to.equal(32);
          });

          it('should be no longer than 32 bytes - Rust', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$entropy = Buffer.alloc(33);

            const result = await validateDocumentsBatchTransitionBasic(
              protocolVersionValidator,
              stateRepositoryMock,
              rawStateTransition,
              executionContext,
            );

            await expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.getInstancePath()).to.equal('/$entropy');
            expect(error.getKeyword()).to.equal('maxItems');
            expect(error.getParams().maxItems).to.equal(32);
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
            protocolVersion: getLatestProtocolVersion(),
            ownerId,
            contractId: dataContract.getId(),
            transitions: documentTransitions.map((t) => t.toObject()),
            signature: Buffer.alloc(65),
            signaturePublicKeyId: 0,
          }, [dataContract.clone()]);

          rawStateTransition = stateTransition.toObject();
        });

        describe('$revision', () => {
          it('should be present - Rust', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            delete documentTransition.$revision;

            const result = await validateDocumentsBatchTransitionBasic(
              protocolVersionValidator,
              stateRepositoryMock,
              rawStateTransition,
              executionContext,
            );

            await expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.getParams().missingProperty).to.equal('$revision');
            expect(error.getKeyword()).to.equal('required');
          });

          it('should be a number - Rust', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$revision = '1';

            const result = await validateDocumentsBatchTransitionBasic(
              protocolVersionValidator,
              stateRepositoryMock,
              rawStateTransition,
              executionContext,
            );

            await expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.getInstancePath()).to.equal('/$revision');
            expect(error.getKeyword()).to.equal('type');
          });

          it('should be multiple of 1.0 - Rust', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$revision = 1.2;

            const result = await validateDocumentsBatchTransitionBasic(
              protocolVersionValidator,
              stateRepositoryMock,
              rawStateTransition,
              executionContext,
            );

            await expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.getInstancePath()).to.equal('/$revision');
            expect(error.getKeyword()).to.equal('type');
          });

          it('should have a minimum value of 1 - Rust', async () => {
            const [documentTransition] = rawStateTransition.transitions;

            documentTransition.$revision = 0;

            const result = await validateDocumentsBatchTransitionBasic(
              protocolVersionValidator,
              stateRepositoryMock,
              rawStateTransition,
              executionContext,
            );

            await expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.getInstancePath()).to.equal('/$revision');
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
            protocolVersion: getLatestProtocolVersion(),
            ownerId,
            contractId: dataContract.getId(),
            transitions: documentTransitions.map((t) => t.toObject()),
            signature: Buffer.alloc(65),
            signaturePublicKeyId: 0,
          }, [dataContract.clone()]);

          rawStateTransition = stateTransition.toObject();
        });

        it('should return invalid result if delete transaction is not valid - Rust', async () => {
          const [documentTransition] = rawStateTransition.transitions;

          delete documentTransition.$id;

          const result = await validateDocumentsBatchTransitionBasic(
            protocolVersionValidator,
            stateRepositoryMock,
            rawStateTransition,
            executionContext,
          );

          await expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.getParams().missingProperty).to.equal('$id');
          expect(error.getKeyword()).to.equal('required');
        });
      });

      it('should return invalid result if there are duplicate unique index values - Rust', async () => {
        const [, , , indexedTransition] = documentTransitions.map((t) => t.toObject());
        const duplicatedTransition = { ...indexedTransition };

        duplicatedTransition.$entropy = Buffer.alloc(32, 'b');
        duplicatedTransition.$id = generateDocumentId(
          dataContract.getId(),
          ownerId,
          duplicatedTransition.$type,
          duplicatedTransition.$entropy,
        );
        duplicatedTransition.firstName = 'Ted';
        const duplicates = [duplicatedTransition, indexedTransition];

        stateTransition = new DocumentsBatchTransition({
          protocolVersion: getLatestProtocolVersion(),
          ownerId,
          contractId: dataContract.getId(),
          transitions: duplicates,
          signature: Buffer.alloc(65),
          signaturePublicKeyId: 0,
        }, [dataContract.clone()]);

        rawStateTransition = stateTransition.toObject();

        const result = await validateDocumentsBatchTransitionBasic(
          protocolVersionValidator,
          stateRepositoryMock,
          rawStateTransition,
          executionContext,
        );

        await expectValidationError(result, DuplicateDocumentTransitionsWithIndicesError);

        const [error] = result.getErrors();

        expect(error.getCode()).to.equal(1020);

        expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
        const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
        expect(fetchDataContractId.toBuffer()).is.deep.equal(dataContract.getId().toBuffer());
      });
    });
  });

  describe('signature', () => {
    it('should be present - Rust', async () => {
      delete rawStateTransition.signature;

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('signature');
    });

    it('should be a byte array - Rust', async () => {
      rawStateTransition.signature = new Array(65).fill('string');

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result, 65);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/signature/0');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be not less than 65 bytes - Rust', async () => {
      rawStateTransition.signature = Buffer.alloc(64);

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/signature');
      expect(error.getKeyword()).to.equal('minItems');
      expect(error.getParams().minItems).to.equal(65);
    });

    it('should be not longer than 96 bytes - Rust', async () => {
      rawStateTransition.signature = Buffer.alloc(97);

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/signature');
      expect(error.getKeyword()).to.equal('maxItems');
      expect(error.getParams().maxItems).to.equal(96);
    });
  });

  describe('signaturePublicKeyId', () => {
    it('should be an integer - Rust', async () => {
      rawStateTransition.signaturePublicKeyId = 1.4;

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/signaturePublicKeyId');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should not be < 0 - Rust', async () => {
      rawStateTransition.signaturePublicKeyId = -1;

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/signaturePublicKeyId');
      expect(error.getKeyword()).to.equal('minimum');
    });
  });

  it('should return valid result - Rust', async () => {
    const result = await validateDocumentsBatchTransitionBasic(
      protocolVersionValidator,
      stateRepositoryMock,
      rawStateTransition,
      executionContext,
    );

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
    const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
    expect(fetchDataContractId.toBuffer()).is.deep.equal(dataContract.getId().toBuffer());
  });

  it('should not validate Document transitions on dry run - Rust', async () => {
    stateRepositoryMock.fetchDataContract.resolves(null);

    executionContext.enableDryRun();

    const result = await validateDocumentsBatchTransitionBasic(
      protocolVersionValidator,
      stateRepositoryMock,
      rawStateTransition,
      executionContext,
    );

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
    const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
    expect(fetchDataContractId.toBuffer()).is.deep.equal(dataContract.getId().toBuffer());
  });
});
