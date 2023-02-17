const { getRE2Class } = require('@dashevo/wasm-re2');

const createAjv = require('@dashevo/dpp/lib/ajv/createAjv');

const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');

const JsonSchemaValidator = require('@dashevo/dpp/lib/validation/JsonSchemaValidator');

const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const enrichDataContractWithBaseSchema = require('@dashevo/dpp/lib/dataContract/enrichDataContractWithBaseSchema');

const DocumentsBatchTransitionJs = require('@dashevo/dpp/lib/document/stateTransition/DocumentsBatchTransition/DocumentsBatchTransition');

const getDocumentTransitionsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentTransitionsFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const ValidationResultJs = require('@dashevo/dpp/lib/validation/ValidationResult');

const validateDocumentsBatchTransitionBasicFactory = require('@dashevo/dpp/lib/document/stateTransition/DocumentsBatchTransition/validation/basic/validateDocumentsBatchTransitionBasicFactory');

const { expectValidationError: expectValidationErrorJs, expectJsonSchemaError: expectJsonSchemaErrorJs } = require('@dashevo/dpp/lib/test/expect/expectError');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const DuplicateDocumentTransitionsWithIndicesErrorJs = require('@dashevo/dpp/lib/errors/consensus/basic/document/DuplicateDocumentTransitionsWithIndicesError');
const DuplicateDocumentTransitionsWithIdsErrorJs = require('@dashevo/dpp/lib/errors/consensus/basic/document/DuplicateDocumentTransitionsWithIdsError');
const SomeConsensusError = require('@dashevo/dpp/lib/test/mocks/SomeConsensusError');
const StateTransitionExecutionContextJs = require('@dashevo/dpp/lib/stateTransition/StateTransitionExecutionContext');

const { default: loadWasmDpp } = require('../../../../../../../dist');
const { expectJsonSchemaError, expectValidationError } = require('../../../../../../../lib/test/expect/expectError');

let DataContract;
let DocumentsBatchTransition;
let StateTransitionExecutionContext;
let validateDocumentsBatchTransitionBasic;
let generateDocumentId;
let MissingDataContractIdError;
let InvalidIdentifierError;
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

describe('validateDocumentsBatchTransitionBasicFactory', () => {
  let dataContractJs;
  let dataContract;
  let documents;
  let rawStateTransitionJs;
  let rawStateTransition;
  let findDuplicatesByIdMock;
  let findDuplicatesByIndicesMock;
  let validateDocumentsBatchTransitionBasicJs;
  let stateTransitionJs;
  let stateTransition;
  let ownerId;
  let stateRepositoryMockJs;
  let stateRepositoryMock;
  let validator;
  let enrichSpy;
  let documentTransitions;
  let validatePartialCompoundIndicesMock;
  let validateProtocolVersionMockJs;
  let executionContextJs;
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
      InvalidIdentifierError,
      MissingDataContractIdError,
      DataContractNotPresentError,
      MissingDocumentTransitionTypeError,
      InvalidDocumentTypeError,
      MissingDocumentTransitionActionError,
      InvalidDocumentTransitionActionError,
      InvalidDocumentTransitionIdError,
      DuplicateDocumentTransitionsWithIndicesError,
      DuplicateDocumentTransitionsWithIdsError,
    } = await loadWasmDpp());

    dataContractJs = getDataContractFixture();
    dataContract = new DataContract(dataContractJs.toObject());

    documents = getDocumentsFixture(dataContractJs);
    ownerId = getDocumentsFixture.ownerId;

    documentTransitions = getDocumentTransitionsFixture({
      create: documents,
    });

    executionContextJs = new StateTransitionExecutionContextJs();
    executionContext = new StateTransitionExecutionContext();

    stateTransitionJs = new DocumentsBatchTransitionJs({
      protocolVersion: protocolVersion.latestVersion,
      ownerId,
      contractId: dataContractJs.getId(),
      transitions: documentTransitions.map((t) => t.toObject()),
      signature: Buffer.alloc(65),
      signaturePublicKeyId: 0,
    }, [dataContractJs]);

    stateTransition = new DocumentsBatchTransition({
      protocolVersion: protocolVersion.latestVersion,
      ownerId,
      contractId: dataContract.getId(),
      transitions: documentTransitions.map((t) => t.toObject()),
      signature: Buffer.alloc(65),
      signaturePublicKeyId: 0,
    }, [dataContract.clone()]);

    rawStateTransitionJs = stateTransitionJs.toObject();
    rawStateTransition = stateTransition.toObject();

    findDuplicatesByIdMock = this.sinonSandbox.stub().returns([]);
    findDuplicatesByIndicesMock = this.sinonSandbox.stub().returns([]);

    const dataContractValidationResult = new ValidationResultJs();
    dataContractValidationResult.setData(dataContractJs);

    stateRepositoryMockJs = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    stateRepositoryMockJs.fetchDataContract.resolves(dataContractJs);
    stateRepositoryMock.fetchDataContract.returns(dataContract.clone());

    const RE2 = await getRE2Class();
    const ajv = createAjv(RE2);

    validator = new JsonSchemaValidator(ajv);

    enrichSpy = this.sinonSandbox.spy(enrichDataContractWithBaseSchema);

    validatePartialCompoundIndicesMock = this.sinonSandbox.stub().returns(
      new ValidationResultJs(),
    );

    validateProtocolVersionMockJs = this.sinonSandbox.stub().returns(new ValidationResultJs());
    protocolVersionValidator = new ProtocolVersionValidator();

    validateDocumentsBatchTransitionBasicJs = validateDocumentsBatchTransitionBasicFactory(
      findDuplicatesByIdMock,
      findDuplicatesByIndicesMock,
      stateRepositoryMockJs,
      validator,
      enrichSpy,
      validatePartialCompoundIndicesMock,
      validateProtocolVersionMockJs,
    );
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {
      delete rawStateTransitionJs.protocolVersion;

      const result = await validateDocumentsBatchTransitionBasicJs(
        rawStateTransitionJs,
        executionContextJs,
      );

      expectJsonSchemaErrorJs(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('protocolVersion');
    });

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

    it('should be an integer', async () => {
      rawStateTransitionJs.protocolVersion = '1';

      const result = await validateDocumentsBatchTransitionBasicJs(
        rawStateTransitionJs,
        executionContextJs,
      );

      expectJsonSchemaErrorJs(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/protocolVersion');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be an integer - Rust', async () => {
      rawStateTransition.protocolVersion = '1';

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/protocolVersion');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be valid', async () => {
      rawStateTransitionJs.protocolVersion = -1;

      const protocolVersionError = new SomeConsensusError('test');
      const protocolVersionResult = new ValidationResultJs([
        protocolVersionError,
      ]);

      validateProtocolVersionMockJs.returns(protocolVersionResult);

      const result = await validateDocumentsBatchTransitionBasicJs(
        rawStateTransitionJs,
        executionContextJs,
      );

      expectValidationErrorJs(result, SomeConsensusError);

      const [error] = result.getErrors();

      expect(error).to.equal(protocolVersionError);

      expect(validateProtocolVersionMockJs).to.be.calledOnceWith(
        rawStateTransitionJs.protocolVersion,
      );
    });

    it('should be valid - Rust', async () => {
      rawStateTransition.protocolVersion = -1;
      try {
        await validateDocumentsBatchTransitionBasic(
          protocolVersionValidator,
          stateRepositoryMock,
          rawStateTransition,
          executionContext,
        );
      } catch (e) {
        expect(e).equal('Error conversion not implemented: unable convert -1 to u64');
      }
    });
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawStateTransitionJs.type;

      const result = await validateDocumentsBatchTransitionBasicJs(
        rawStateTransitionJs,
        executionContextJs,
      );

      expectJsonSchemaErrorJs(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('type');
    });

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

    it('should be equal 1', async () => {
      rawStateTransitionJs.type = 666;

      const result = await validateDocumentsBatchTransitionBasicJs(
        rawStateTransitionJs,
        executionContextJs,
      );

      expectJsonSchemaErrorJs(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/type');
      expect(error.getKeyword()).to.equal('const');
      expect(error.getParams().allowedValue).to.equal(1);
    });

    it('should be equal 1', async () => {
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
    it('should be present', async () => {
      delete rawStateTransitionJs.ownerId;

      const result = await validateDocumentsBatchTransitionBasicJs(
        rawStateTransitionJs,
        executionContextJs,
      );

      expectJsonSchemaErrorJs(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('ownerId');
    });

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

    it('should be a byte array', async () => {
      rawStateTransitionJs.ownerId = new Array(32).fill('string');

      const result = await validateDocumentsBatchTransitionBasicJs(
        rawStateTransitionJs,
        executionContextJs,
      );

      expectJsonSchemaErrorJs(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/ownerId/0');
      expect(error.getKeyword()).to.equal('type');

      expect(byteArrayError.getKeyword()).to.equal('byteArray');
    });

    it('should be a byte array', async () => {
      rawStateTransition.ownerId = new Array(32).fill('string');

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result, 32);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/ownerId/0');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be no less than 32 bytes', async () => {
      rawStateTransitionJs.ownerId = Buffer.alloc(31);

      const result = await validateDocumentsBatchTransitionBasicJs(
        rawStateTransitionJs,
        executionContextJs,
      );

      expectJsonSchemaErrorJs(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/ownerId');
      expect(error.getKeyword()).to.equal('minItems');
    });

    it('should be no less than 32 bytes - Rust', async () => {
      rawStateTransition.ownerId = Buffer.alloc(31);

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/ownerId');
      expect(error.getKeyword()).to.equal('minItems');
    });

    it('should be no longer than 32 bytes', async () => {
      rawStateTransitionJs.ownerId = Buffer.alloc(33);

      const result = await validateDocumentsBatchTransitionBasicJs(
        rawStateTransitionJs,
        executionContextJs,
      );

      expectJsonSchemaErrorJs(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/ownerId');
      expect(error.getKeyword()).to.equal('maxItems');
    });

    it('should be no longer than 32 bytes - Rust', async () => {
      rawStateTransition.ownerId = Buffer.alloc(33);

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/ownerId');
      expect(error.getKeyword()).to.equal('maxItems');
    });
  });

  describe('document transitions', () => {
    it('should be present', async () => {
      delete rawStateTransitionJs.transitions;

      const result = await validateDocumentsBatchTransitionBasicJs(
        rawStateTransitionJs,
        executionContextJs,
      );

      expectJsonSchemaErrorJs(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('transitions');
    });

    it('should be present Rust', async () => {
      delete rawStateTransition.transitions;

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
      expect(error.getParams().missingProperty).to.equal('transitions');
    });

    it('should be an array', async () => {
      rawStateTransitionJs.transitions = {};

      const result = await validateDocumentsBatchTransitionBasicJs(
        rawStateTransitionJs,
        executionContextJs,
      );

      expectJsonSchemaErrorJs(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/transitions');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be an array', async () => {
      rawStateTransition.transitions = {};

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/transitions');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should have at least one element', async () => {
      rawStateTransitionJs.transitions = [];

      const result = await validateDocumentsBatchTransitionBasicJs(
        rawStateTransitionJs,
        executionContextJs,
      );

      expectJsonSchemaErrorJs(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/transitions');
      expect(error.getKeyword()).to.equal('minItems');
      expect(error.getParams().limit).to.equal(1);
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

    it('should have no more than 10 elements', async () => {
      rawStateTransitionJs.transitions = Array(11).fill({});

      const result = await validateDocumentsBatchTransitionBasicJs(
        rawStateTransitionJs,
        executionContextJs,
      );

      expectJsonSchemaErrorJs(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/transitions');
      expect(error.getKeyword()).to.equal('maxItems');
      expect(error.getParams().limit).to.equal(10);
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

    it('should have objects as elements', async () => {
      rawStateTransitionJs.transitions = [1];

      const result = await validateDocumentsBatchTransitionBasicJs(
        rawStateTransitionJs,
        executionContextJs,
      );

      expectJsonSchemaErrorJs(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/transitions/0');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should have objects as elements - Rust', async () => {
      rawStateTransition.transitions = [1];

      const result = await validateDocumentsBatchTransitionBasic(
        protocolVersionValidator,
        stateRepositoryMock,
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/transitions/0');
      expect(error.getKeyword()).to.equal('type');
    });

    describe('document transition', () => {
      describe('$id', () => {
        it('should be present', async () => {
          const [documentTransition] = rawStateTransitionJs.transitions;

          delete documentTransition.$id;

          const result = await validateDocumentsBatchTransitionBasicJs(
            rawStateTransitionJs,
            executionContextJs,
          );

          expectJsonSchemaErrorJs(result);

          const [error] = result.getErrors();

          expect(error.instancePath).to.equal('');
          expect(error.getKeyword()).to.equal('required');
          expect(error.getParams().missingProperty).to.equal('$id');
        });

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

        it('should be a byte array', async () => {
          const [documentTransition] = rawStateTransitionJs.transitions;

          documentTransition.$id = new Array(32).fill('string');

          const result = await validateDocumentsBatchTransitionBasicJs(
            rawStateTransitionJs,
            executionContextJs,
          );

          expectJsonSchemaErrorJs(result, 2);

          const [error, byteArrayError] = result.getErrors();

          expect(error.instancePath).to.equal('/$id/0');
          expect(error.getKeyword()).to.equal('type');

          expect(byteArrayError.getKeyword()).to.equal('byteArray');
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

          await expectJsonSchemaError(result, 32);

          const [error] = result.getErrors();

          expect(error.getInstancePath()).to.equal('/$id/0');
          expect(error.getKeyword()).to.equal('type');
        });

        it('should be no less than 32 bytes', async () => {
          const [documentTransition] = rawStateTransitionJs.transitions;

          documentTransition.$id = Buffer.alloc(31);

          const result = await validateDocumentsBatchTransitionBasicJs(
            rawStateTransitionJs,
            executionContextJs,
          );

          expectJsonSchemaErrorJs(result);

          const [error] = result.getErrors();

          expect(error.instancePath).to.equal('/$id');
          expect(error.getKeyword()).to.equal('minItems');
          expect(error.getParams().limit).to.equal(32);
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

          await expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.getInstancePath()).to.equal('/$id');
          expect(error.getKeyword()).to.equal('minItems');
          expect(error.getParams().minItems).to.equal(32);
        });

        it('should be no longer than 32 bytes', async () => {
          const [documentTransition] = rawStateTransitionJs.transitions;

          documentTransition.$id = Buffer.alloc(33);

          const result = await validateDocumentsBatchTransitionBasicJs(
            rawStateTransitionJs,
            executionContextJs,
          );

          expectJsonSchemaErrorJs(result);

          const [error] = result.getErrors();

          expect(error.instancePath).to.equal('/$id');
          expect(error.getKeyword()).to.equal('maxItems');
          expect(error.getParams().limit).to.equal(32);
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
          await expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.getInstancePath()).to.equal('/$id');
          expect(error.getKeyword()).to.equal('maxItems');
          expect(error.getParams().maxItems).to.equal(32);
        });

        it('should no have duplicate IDs in the state transition', async () => {
          const duplicates = [documentTransitions[0].toObject()];

          findDuplicatesByIdMock.returns(duplicates);

          const result = await validateDocumentsBatchTransitionBasicJs(
            rawStateTransitionJs,
            executionContextJs,
          );

          expectValidationErrorJs(result, DuplicateDocumentTransitionsWithIdsErrorJs);

          const [error] = result.getErrors();

          expect(error.getCode()).to.equal(1019);
          expect(error.getDocumentTransitionReferences()).to.deep.equal(
            duplicates.map((d) => [d.$type, d.$id]),
          );

          expect(stateRepositoryMockJs.fetchDataContract).to.have.been.calledOnceWithExactly(
            dataContractJs.getId(),
            executionContextJs,
          );
          expect(enrichSpy).to.have.been.calledThrice();
          expect(findDuplicatesByIdMock).to.have.been.calledOnceWithExactly(
            rawStateTransitionJs.transitions,
          );
          expect(findDuplicatesByIndicesMock).to.have.been.calledOnceWithExactly(
            rawStateTransitionJs.transitions, dataContractJs,
          );
        });

        it('should no have duplicate IDs in the state transition - Rust', async () => {
          const [documentTransition] = documentTransitions.map((t) => t.toObject());

          stateTransition = new DocumentsBatchTransition({
            protocolVersion: protocolVersion.latestVersion,
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

          firstDocumentTransition.$dataContractId = Buffer.alloc(2);

          const result = await validateDocumentsBatchTransitionBasic(
            protocolVersionValidator,
            stateRepositoryMock,
            rawStateTransition,
            executionContext,
          );

          await expectValidationError(result, InvalidIdentifierError);

          const [error] = result.getErrors();

          expect(error.getCode()).to.equal(1006);

          expect(error.getIdentifierName()).to.equal('$dataContractId');
          expect(error.getIdentifierError()).to.equal('Identifier Error: Identifier must be 32 bytes long');

          expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
          const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
          expect(fetchDataContractId.toBuffer()).is.deep.equal(dataContract.getId().toBuffer());
        });

        it('should exists in the state - Rust', async () => {
          stateRepositoryMock.fetchDataContract.returns(undefined);

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
            firstTransition.$id = generateRandomIdentifier().toBuffer();

            const result = await validateDocumentsBatchTransitionBasic(
              protocolVersionValidator,
              stateRepositoryMock,
              rawStateTransition,
              executionContext,
            );

            const [error] = result.getErrors();
            console.log(error.toString());

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

          stateTransitionJs = new DocumentsBatchTransitionJs({
            protocolVersion: protocolVersion.latestVersion,
            ownerId,
            contractId: dataContractJs.getId(),
            transitions: documentTransitions.map((t) => t.toObject()),
            signature: Buffer.alloc(65),
            signaturePublicKeyId: 0,
          }, [dataContractJs]);

          stateTransition = new DocumentsBatchTransition({
            protocolVersion: protocolVersion.latestVersion,
            ownerId,
            contractId: dataContract.getId(),
            transitions: documentTransitions.map((t) => t.toObject()),
            signature: Buffer.alloc(65),
            signaturePublicKeyId: 0,
          }, [dataContract.clone()]);

          rawStateTransitionJs = stateTransitionJs.toObject();
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

          stateTransitionJs = new DocumentsBatchTransitionJs({
            protocolVersion: protocolVersion.latestVersion,
            ownerId,
            contractId: dataContractJs.getId(),
            transitions: documentTransitions.map((t) => t.toObject()),
            signature: Buffer.alloc(65),
            signaturePublicKeyId: 0,
          }, [dataContractJs]);

          stateTransition = new DocumentsBatchTransition({
            protocolVersion: protocolVersion.latestVersion,
            ownerId,
            contractId: dataContract.getId(),
            transitions: documentTransitions.map((t) => t.toObject()),
            signature: Buffer.alloc(65),
            signaturePublicKeyId: 0,
          }, [dataContract.clone()]);

          rawStateTransitionJs = stateTransitionJs.toObject();
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

      it('should return invalid result if there are duplicate unique index values', async () => {
        const duplicates = [documentTransitions[1].toObject()];

        findDuplicatesByIndicesMock.returns(duplicates);

        const result = await validateDocumentsBatchTransitionBasicJs(
          rawStateTransitionJs,
          executionContextJs,
        );

        expectValidationErrorJs(result, DuplicateDocumentTransitionsWithIndicesErrorJs);

        const [error] = result.getErrors();

        expect(error.getCode()).to.equal(1020);

        expect(error.getDocumentTransitionReferences()).to.deep.equal(
          duplicates.map((d) => [d.$type, d.$id]),
        );

        expect(stateRepositoryMockJs.fetchDataContract).to.have.been.calledOnceWithExactly(
          dataContractJs.getId(),
          executionContextJs,
        );
        expect(enrichSpy).to.have.been.calledThrice();
        expect(findDuplicatesByIdMock).to.have.been.calledOnceWithExactly(
          rawStateTransitionJs.transitions,
        );
        expect(findDuplicatesByIndicesMock).to.have.been.calledOnceWithExactly(
          rawStateTransitionJs.transitions, dataContractJs,
        );
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
        const duplicates = [duplicatedTransition, indexedTransition];

        stateTransition = new DocumentsBatchTransition({
          protocolVersion: protocolVersion.latestVersion,
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

      it('should return invalid result if compound index doesn\'t contain all fields ', async () => {
        const consensusError = new SomeConsensusError('error');

        validatePartialCompoundIndicesMock.returns(
          new ValidationResultJs([consensusError]),
        );

        const result = await validateDocumentsBatchTransitionBasicJs(
          rawStateTransitionJs,
          executionContextJs,
        );

        expectValidationErrorJs(result);

        const [error] = result.getErrors();

        expect(error).to.equal(consensusError);

        expect(validatePartialCompoundIndicesMock).to.be.calledOnceWithExactly(
          ownerId.toBuffer(),
          rawStateTransitionJs.transitions,
          dataContractJs,
        );

        expect(stateRepositoryMockJs.fetchDataContract).to.have.been.calledOnceWithExactly(
          dataContractJs.getId(),
          executionContextJs,
        );
        expect(enrichSpy).to.have.been.calledThrice();
      });

      it('should return invalid result if compound index doesn\'t contain all fields - Rust', async () => {
        const consensusError = new SomeConsensusError('error');

        validatePartialCompoundIndicesMock.returns(
          new ValidationResultJs([consensusError]),
        );

        const result = await validateDocumentsBatchTransitionBasicJs(
          rawStateTransitionJs,
          executionContextJs,
        );

        expectValidationErrorJs(result);

        const [error] = result.getErrors();

        expect(error).to.equal(consensusError);

        expect(validatePartialCompoundIndicesMock).to.be.calledOnceWithExactly(
          ownerId.toBuffer(),
          rawStateTransitionJs.transitions,
          dataContractJs,
        );

        expect(stateRepositoryMockJs.fetchDataContract).to.have.been.calledOnceWithExactly(
          dataContractJs.getId(),
          executionContextJs,
        );
        expect(enrichSpy).to.have.been.calledThrice();
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

    it('should be not less than 65 bytes', async () => {
      rawStateTransitionJs.signature = Buffer.alloc(64);

      const result = await validateDocumentsBatchTransitionBasicJs(
        rawStateTransitionJs,
        executionContextJs,
      );

      expectJsonSchemaErrorJs(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/signature');
      expect(error.getKeyword()).to.equal('minItems');
      expect(error.getParams().limit).to.equal(65);
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
    stateRepositoryMock.fetchDataContract.returns(null);

    executionContext.enableDryRun();

    const result = await validateDocumentsBatchTransitionBasic(
      protocolVersionValidator,
      stateRepositoryMock,
      rawStateTransition,
      executionContext,
    );

    executionContextJs.disableDryRun();

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
    const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
    expect(fetchDataContractId.toBuffer()).is.deep.equal(dataContract.getId().toBuffer());
  });
});
