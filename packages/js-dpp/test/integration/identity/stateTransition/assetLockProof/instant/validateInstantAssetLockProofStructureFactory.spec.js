const rewiremock = require('rewiremock/node');

const { Transaction } = require('@dashevo/dashcore-lib');

const createAjv = require('../../../../../../lib/ajv/createAjv');

const getInstantAssetLockFixture = require('../../../../../../lib/test/fixtures/getInstantAssetLockProofFixture');
const JsonSchemaValidator = require('../../../../../../lib/validation/JsonSchemaValidator');
const createStateRepositoryMock = require('../../../../../../lib/test/mocks/createStateRepositoryMock');
const InvalidIdentityAssetLockProofError = require('../../../../../../lib/errors/InvalidIdentityAssetLockProofError');
const IdentityAssetLockProofMismatchError = require('../../../../../../lib/errors/IdentityAssetLockProofMismatchError');
const InvalidIdentityAssetLockProofSignatureError = require('../../../../../../lib/errors/InvalidIdentityAssetLockProofSignatureError');

const { expectValidationError, expectJsonSchemaError } = require(
  '../../../../../../lib/test/expect/expectError',
);

const ValidationResult = require('../../../../../../lib/validation/ValidationResult');
const InvalidIdentityAssetLockTransactionError = require('../../../../../../lib/errors/InvalidIdentityAssetLockTransactionError');

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

  beforeEach(function beforeEach() {
    const assetLock = getInstantAssetLockFixture();
    transaction = assetLock.getTransaction();

    rawProof = assetLock.toObject();

    jsonSchemaValidator = new JsonSchemaValidator(createAjv());

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
      '../../../../../../lib/identity/stateTransitions/assetLockProof/instant/validateInstantAssetLockProofStructureFactory',
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

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('type');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
    });

    it('should be equal to 0', async () => {
      rawProof.type = -1;

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/type');
      expect(error.keyword).to.equal('const');

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

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('instantLock');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
    });

    it('should be a byte array', async () => {
      rawProof.instantLock = new Array(165).fill('string');

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.dataPath).to.equal('/instantLock/0');
      expect(error.keyword).to.equal('type');

      expect(byteArrayError.keyword).to.equal('byteArray');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
    });

    it('should not be shorter than 160 bytes', async () => {
      rawProof.instantLock = Buffer.alloc(159);

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/instantLock');
      expect(error.keyword).to.equal('minItems');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
    });

    it('should not be longer than 100 Kb', async () => {
      rawProof.instantLock = Buffer.alloc(100001);

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/instantLock');
      expect(error.keyword).to.equal('maxItems');

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

      expect(error.message).to.equal(`Invalid asset lock proof: ${instantLockError.message}`);

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
    });

    it('should lock the same transaction', async () => {
      instantLockMock.txid = '123';

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectValidationError(result, IdentityAssetLockProofMismatchError);

      expect(stateRepositoryMock.verifyInstantLock).to.be.calledOnce();
      expect(instantLockMock.verify).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.be.calledOnce();
    });

    it('should have valid signature', async () => {
      stateRepositoryMock.verifyInstantLock.resolves(false);

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectValidationError(result, InvalidIdentityAssetLockProofSignatureError);

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

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('transaction');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
    });

    it('should be a byte array', async () => {
      rawProof.transaction = new Array(65).fill('string');

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.dataPath).to.equal('/transaction/0');
      expect(error.keyword).to.equal('type');

      expect(byteArrayError.keyword).to.equal('byteArray');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
    });

    it('should not be shorter than 1 byte', async () => {
      rawProof.transaction = Buffer.alloc(0);

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/transaction');
      expect(error.keyword).to.equal('minItems');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
    });

    it('should not be longer than 100 Kb', async () => {
      rawProof.transaction = Buffer.alloc(100001);

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/transaction');
      expect(error.keyword).to.equal('maxItems');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
    });

    it('should should be valid', async () => {
      validateAssetLockTransactionResult.addError(new InvalidIdentityAssetLockTransactionError('Unknown special transaction type'));
      validateAssetLockTransactionMock.resolves(validateAssetLockTransactionResult);

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectValidationError(result, InvalidIdentityAssetLockTransactionError);

      const [error] = result.getErrors();

      expect(error.message).to.equal('Invalid asset lock transaction: Unknown special transaction type');

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

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('outputIndex');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
    });

    it('should be an integer', async () => {
      rawProof.outputIndex = 1.1;

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/outputIndex');
      expect(error.keyword).to.equal('type');

      expect(stateRepositoryMock.verifyInstantLock).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
    });

    it('should not be less than 0', async () => {
      rawProof.outputIndex = -1;

      const result = await validateInstantAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/outputIndex');
      expect(error.keyword).to.equal('minimum');

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
