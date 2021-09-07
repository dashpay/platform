const { default: getRE2Class } = require('@dashevo/re2-wasm');

const rewiremock = require('rewiremock/node');

const { Transaction } = require('@dashevo/dashcore-lib');

const createAjv = require('../../../../../../lib/ajv/createAjv');

const getInstantAssetLockFixture = require('../../../../../../lib/test/fixtures/getInstantAssetLockProofFixture');
const JsonSchemaValidator = require('../../../../../../lib/validation/JsonSchemaValidator');
const createStateRepositoryMock = require('../../../../../../lib/test/mocks/createStateRepositoryMock');
const InvalidIdentityAssetLockProofError = require('../../../../../../lib/errors/consensus/basic/identity/InvalidInstantAssetLockProofError');
const IdentityAssetLockProofLockedTransactionMismatchError = require('../../../../../../lib/errors/consensus/basic/identity/IdentityAssetLockProofLockedTransactionMismatchError');
const InvalidIdentityAssetLockProofSignatureError = require('../../../../../../lib/errors/consensus/basic/identity/InvalidInstantAssetLockProofSignatureError');

const { expectValidationError, expectJsonSchemaError } = require(
  '../../../../../../lib/test/expect/expectError',
);

const ValidationResult = require('../../../../../../lib/validation/ValidationResult');
const InvalidIdentityAssetLockTransactionError = require('../../../../../../lib/errors/consensus/basic/identity/InvalidIdentityAssetLockTransactionError');

describe('validateInstantAssetLockProofStructureFactory', () => {
  let rawProof;
  let transaction;
  let stateRepositoryMock;
  let InstantLockClassMock;
  let instantLockMock;
  let validateInstantAssetLockProofStructure;
  let jsonSchemaValidator;
  let validateInstantAssetLockProofStructureFactory;
  let validateAssetLockTransactionResult;
  let publicKeyHash;
  let validateAssetLockTransactionMock;

  beforeEach(async function beforeEach() {
    const assetLock = getInstantAssetLockFixture();
    transaction = assetLock.getTransaction();

    rawProof = assetLock.toObject();

    const RE2 = await getRE2Class();
    const ajv = createAjv(RE2);

    jsonSchemaValidator = new JsonSchemaValidator(ajv);

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.verifyInstantLock.resolves(true);

    instantLockMock = {
      txid: transaction.id,
      verify: this.sinonSandbox.stub().resolves(true),
    };

    InstantLockClassMock = {
      fromBuffer: this.sinonSandbox.stub().returns(instantLockMock),
    };

    validateInstantAssetLockProofStructureFactory = rewiremock.proxy(
      '../../../../../../lib/identity/stateTransition/assetLockProof/instant/validateInstantAssetLockProofStructureFactory',
      {
        '../../../../../../node_modules/@dashevo/dashcore-lib': {
          InstantLock: InstantLockClassMock,
          Transaction,
        },
      },
    );

    publicKeyHash = Buffer.from('152073ca2300a86b510fa2f123d3ea7da3af68dc', 'hex');

    validateAssetLockTransactionResult = new ValidationResult();
    validateAssetLockTransactionResult.setData({
      publicKeyHash,
      transaction,
    });
    validateAssetLockTransactionMock = this.sinonSandbox.stub().resolves(
      validateAssetLockTransactionResult,
    );

    validateInstantAssetLockProofStructure = validateInstantAssetLockProofStructureFactory(
      jsonSchemaValidator,
      stateRepositoryMock,
      validateAssetLockTransactionMock,
    );
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawProof.type;

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('type');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
    });

    it('should be equal to 0', async () => {
      rawProof.type = -1;

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/type');
      expect(error.getKeyword()).to.equal('const');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
    });
  });

  describe('instantLock', () => {
    it('should be present', async () => {
      delete rawProof.instantLock;

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('instantLock');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
    });

    it('should be a byte array', async () => {
      rawProof.instantLock = new Array(165).fill('string');

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/instantLock/0');
      expect(error.getKeyword()).to.equal('type');

      expect(byteArrayError.getKeyword()).to.equal('byteArray');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
    });

    it('should not be shorter than 160 bytes', async () => {
      rawProof.instantLock = Buffer.alloc(159);

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/instantLock');
      expect(error.getKeyword()).to.equal('minItems');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
    });

    it('should not be longer than 100 Kb', async () => {
      rawProof.instantLock = Buffer.alloc(100001);

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/instantLock');
      expect(error.getKeyword()).to.equal('maxItems');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
    });

    it('should be valid', async () => {
      const instantLockError = new Error('something is wrong');

      InstantLockClassMock.fromBuffer.throws(instantLockError);

      rawProof.instantLock = Buffer.alloc(200);

      const result = await validateInstantAssetLockProofStructure(rawProof);
      expectValidationError(result, InvalidIdentityAssetLockProofError);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(1041);
      expect(error.getValidationError()).to.equal(instantLockError);

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
    });

    it('should lock the same transaction', async () => {
      const txId = Buffer.alloc(32);
      instantLockMock.txid = txId.toString('hex');

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectValidationError(result, IdentityAssetLockProofLockedTransactionMismatchError);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(1031);
      expect(error.getInstantLockTransactionId()).to.deep.equal(txId);
      expect(error.getAssetLockTransactionId()).to.deep.equal(Buffer.from(transaction.id, 'hex'));

      expect(stateRepositoryMock.verifyInstantLock).to.be.calledOnce();
      expect(instantLockMock.verify).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.be.calledOnce();
    });

    it('should have valid signature', async () => {
      stateRepositoryMock.verifyInstantLock.resolves(false);

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectValidationError(result, InvalidIdentityAssetLockProofSignatureError);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(1042);

      expect(stateRepositoryMock.verifyInstantLock).to.be.calledOnce();
      expect(validateAssetLockTransactionMock).to.not.be.called();
    });
  });

  describe('transaction', () => {
    it('should be present', async () => {
      delete rawProof.transaction;

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('transaction');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
    });

    it('should be a byte array', async () => {
      rawProof.transaction = new Array(65).fill('string');

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/transaction/0');
      expect(error.getKeyword()).to.equal('type');

      expect(byteArrayError.getKeyword()).to.equal('byteArray');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
    });

    it('should not be shorter than 1 byte', async () => {
      rawProof.transaction = Buffer.alloc(0);

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/transaction');
      expect(error.getKeyword()).to.equal('minItems');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
    });

    it('should not be longer than 100 Kb', async () => {
      rawProof.transaction = Buffer.alloc(100001);

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/transaction');
      expect(error.getKeyword()).to.equal('maxItems');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
    });

    it('should should be valid', async () => {
      const validationError = new Error('parsing failed');

      const consensusError = new InvalidIdentityAssetLockTransactionError(validationError.message);

      consensusError.setValidationError(validationError);

      validateAssetLockTransactionResult.addError(consensusError);
      validateAssetLockTransactionMock.resolves(validateAssetLockTransactionResult);

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectValidationError(result, InvalidIdentityAssetLockTransactionError);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(1038);

      expect(error).to.equal(consensusError);

      expect(error.getValidationError()).to.equal(validationError);

      expect(stateRepositoryMock.verifyInstantLock).to.be.calledOnce();
      expect(instantLockMock.verify).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.be.calledOnce();
    });
  });

  describe('outputIndex', () => {
    it('should be present', async () => {
      delete rawProof.outputIndex;

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('outputIndex');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
    });

    it('should be an integer', async () => {
      rawProof.outputIndex = 1.1;

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/outputIndex');
      expect(error.getKeyword()).to.equal('type');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
    });

    it('should not be less than 0', async () => {
      rawProof.outputIndex = -1;

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/outputIndex');
      expect(error.getKeyword()).to.equal('minimum');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
    });
  });

  it('should return valid result', async () => {
    const result = await validateInstantAssetLockProofStructure(rawProof);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
    expect(result.getData()).to.deep.equal(publicKeyHash);

    expect(stateRepositoryMock.verifyInstantLock).to.be.calledOnce();
  });
});
