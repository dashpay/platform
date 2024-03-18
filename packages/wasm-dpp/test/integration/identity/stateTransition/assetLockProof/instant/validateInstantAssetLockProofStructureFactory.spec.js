const DashCoreLib = require('@dashevo/dashcore-lib');
const { expect } = require('chai');

const getInstantAssetLockFixture = require('../../../../../../lib/test/fixtures/getInstantAssetLockProofFixture');
const createStateRepositoryMock = require('../../../../../../lib/test/mocks/createStateRepositoryMock');
const { expectJsonSchemaError, expectValidationError } = require('../../../../../../lib/test/expect/expectError');

const { default: loadWasmDpp } = require('../../../../../../dist');

describe.skip('validateInstantAssetLockProofStructureFactory', () => {
  let rawProof;
  let transaction;
  let stateRepositoryMock;
  let executionContext;

  let StateTransitionExecutionContext;
  let ValidationResult;
  let InvalidInstantAssetLockProofError;
  let IdentityAssetLockProofLockedTransactionMismatchError;
  let InvalidInstantAssetLockProofSignatureError;
  let InvalidIdentityAssetLockTransactionError;
  let InstantAssetLockProofStructureValidator;

  let validateInstantAssetLockProofStructure;

  before(async () => {
    ({
      StateTransitionExecutionContext,
      ValidationResult,
      InvalidInstantAssetLockProofError,
      IdentityAssetLockProofLockedTransactionMismatchError,
      InvalidInstantAssetLockProofSignatureError,
      InvalidIdentityAssetLockTransactionError,
      InstantAssetLockProofStructureValidator,
    } = await loadWasmDpp());
  });

  beforeEach(async function beforeEach() {
    const assetLock = await getInstantAssetLockFixture();
    transaction = assetLock.getTransaction();

    rawProof = assetLock.toObject();

    stateRepositoryMock = createStateRepositoryMock(this.sinon);
    stateRepositoryMock.verifyInstantLock.resolves(true);
    stateRepositoryMock.isAssetLockTransactionOutPointAlreadyUsed.resolves(false);

    executionContext = new StateTransitionExecutionContext();

    const validator = new InstantAssetLockProofStructureValidator(stateRepositoryMock);
    validateInstantAssetLockProofStructure = (proof, context) => validator.validate(
      proof,
      context,
    );
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawProof.type;

      const result = await validateInstantAssetLockProofStructure(rawProof, executionContext);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('type');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
    });

    it('should be equal to 0', async () => {
      rawProof.type = -1;

      const result = await validateInstantAssetLockProofStructure(rawProof, executionContext);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/type');
      expect(error.getKeyword()).to.equal('const');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
    });
  });

  describe('instantLock', () => {
    it('should be present', async () => {
      delete rawProof.instantLock;

      const result = await validateInstantAssetLockProofStructure(rawProof, executionContext);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('instantLock');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
    });

    it('should be a byte array', async () => {
      rawProof.instantLock = new Array(165).fill('string');

      const result = await validateInstantAssetLockProofStructure(rawProof, executionContext);

      await expectJsonSchemaError(result, 165);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/instantLock/0');
      expect(error.getKeyword()).to.equal('type');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
    });

    it('should not be shorter than 160 bytes', async () => {
      rawProof.instantLock = Buffer.alloc(159);

      const result = await validateInstantAssetLockProofStructure(rawProof, executionContext);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/instantLock');
      expect(error.getKeyword()).to.equal('minItems');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
    });

    it('should not be longer than 100 Kb', async () => {
      rawProof.instantLock = Buffer.alloc(100001);

      const result = await validateInstantAssetLockProofStructure(rawProof, executionContext);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/instantLock');
      expect(error.getKeyword()).to.equal('maxItems');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
    });

    it('should be valid', async () => {
      rawProof.instantLock = Buffer.alloc(200);

      const result = await validateInstantAssetLockProofStructure(rawProof, executionContext);
      await expectValidationError(result, InvalidInstantAssetLockProofError);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(1041);

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
    });

    it('should lock the same transaction', async () => {
      const txId = Buffer.alloc(32);

      const instantLockParsed = new DashCoreLib.InstantLock(rawProof.instantLock);
      instantLockParsed.txid = txId.toString('hex');
      rawProof.instantLock = instantLockParsed.toBuffer();

      const result = await validateInstantAssetLockProofStructure(rawProof, executionContext);

      await expectValidationError(
        result,
        IdentityAssetLockProofLockedTransactionMismatchError,
      );

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(1031);
      expect(error.getInstantLockTransactionId()).to.deep.equal(txId);
      expect(error.getAssetLockTransactionId()).to.deep.equal(Buffer.from(transaction.id, 'hex'));

      expect(stateRepositoryMock.verifyInstantLock).to.be.calledOnce();
    });

    it('should have valid signature', async () => {
      stateRepositoryMock.verifyInstantLock.resolves(false);

      const result = await validateInstantAssetLockProofStructure(rawProof, executionContext);

      await expectValidationError(
        result,
        InvalidInstantAssetLockProofSignatureError,
      );

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(1042);

      expect(stateRepositoryMock.verifyInstantLock).to.be.calledOnce();
    });
  });

  describe('transaction', () => {
    it('should be present', async () => {
      delete rawProof.transaction;

      const result = await validateInstantAssetLockProofStructure(rawProof, executionContext);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('transaction');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
    });

    it('should be a byte array', async () => {
      rawProof.transaction = new Array(65).fill('string');

      const result = await validateInstantAssetLockProofStructure(rawProof, executionContext);

      await expectJsonSchemaError(result, 65);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/transaction/0');
      expect(error.getKeyword()).to.equal('type');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
    });

    it('should not be shorter than 1 byte', async () => {
      rawProof.transaction = Buffer.alloc(0);

      const result = await validateInstantAssetLockProofStructure(rawProof, executionContext);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/transaction');
      expect(error.getKeyword()).to.equal('minItems');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
    });

    it('should not be longer than 100 Kb', async () => {
      rawProof.transaction = Buffer.alloc(100001);

      const result = await validateInstantAssetLockProofStructure(rawProof, executionContext);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/transaction');
      expect(error.getKeyword()).to.equal('maxItems');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
    });

    it('should should be valid', async () => {
      rawProof.transaction = Buffer.alloc(1000);

      const result = await validateInstantAssetLockProofStructure(rawProof, executionContext);

      await expectValidationError(result, InvalidIdentityAssetLockTransactionError);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(1038);
      expect(error.getErrorMessage()).to.be.not.empty();

      expect(stateRepositoryMock.verifyInstantLock).to.be.calledOnce();
    });
  });

  describe('outputIndex', () => {
    it('should be present', async () => {
      delete rawProof.outputIndex;

      const result = await validateInstantAssetLockProofStructure(rawProof, executionContext);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('outputIndex');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
    });

    it('should be an integer', async () => {
      rawProof.outputIndex = 1.1;

      const result = await validateInstantAssetLockProofStructure(rawProof, executionContext);

      await expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/outputIndex');
      expect(error.getKeyword()).to.equal('type');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
    });

    it('should not be less than 0', async () => {
      rawProof.outputIndex = -1;

      const result = await validateInstantAssetLockProofStructure(rawProof, executionContext);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/outputIndex');
      expect(error.getKeyword()).to.equal('minimum');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
    });
  });

  it('should return valid result', async () => {
    const result = await validateInstantAssetLockProofStructure(
      rawProof,
      executionContext,
    );

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    const publicKeyHash = transaction.outputs[0].script.getData();
    expect(result.getData()).to.deep.equal(publicKeyHash);

    expect(stateRepositoryMock.verifyInstantLock).to.be.calledOnce();
  });
});
