const { Transaction } = require('@dashevo/dashcore-lib');

const createAjv = require('../../../../../lib/ajv/createAjv');

const JsonSchemaValidator = require('../../../../../lib/validation/JsonSchemaValidator');

const validateAssetLockStructureFactory = require('../../../../../lib/identity/stateTransitions/assetLock/validateAssetLockStructureFactory');

const getAssetLockFixture = require('../../../../../lib/test/fixtures/getAssetLockFixture');

const InvalidIdentityAssetLockTransactionOutputError = require(
  '../../../../../lib/errors/InvalidIdentityAssetLockTransactionOutputError',
);
const InvalidIdentityAssetLockTransactionError = require('../../../../../lib/errors/InvalidIdentityAssetLockTransactionError');
const IdentityAssetLockTransactionOutputNotFoundError = require('../../../../../lib/errors/IdentityAssetLockTransactionOutputNotFoundError');
const { expectValidationError, expectJsonSchemaError } = require(
  '../../../../../lib/test/expect/expectError',
);

const ConsensusError = require('../../../../../lib/errors/ConsensusError');
const ValidationResult = require('../../../../../lib/validation/ValidationResult');

describe('validateAssetLockStructureFactory', () => {
  let validateAssetLockStructure;
  let assetLock;
  let rawAssetLock;
  let proofValidationFunctionMock;

  beforeEach(function beforeEach() {
    assetLock = getAssetLockFixture();

    rawAssetLock = assetLock.toObject();

    const jsonSchemaValidator = new JsonSchemaValidator(createAjv());

    proofValidationFunctionMock = this.sinonSandbox.stub();

    const proofValidationFunctionsByType = {
      0: proofValidationFunctionMock,
    };

    validateAssetLockStructure = validateAssetLockStructureFactory(
      jsonSchemaValidator,
      proofValidationFunctionsByType,
    );
  });

  describe('transaction', () => {
    it('should be present', async () => {
      delete rawAssetLock.transaction;

      const result = await validateAssetLockStructure(rawAssetLock);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('transaction');
    });

    it('should be a byte array', async () => {
      rawAssetLock.transaction = new Array(65).fill('string');

      const result = await validateAssetLockStructure(rawAssetLock);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.dataPath).to.equal('.transaction[0]');
      expect(error.keyword).to.equal('type');

      expect(byteArrayError.keyword).to.equal('byteArray');
    });

    it('should be not shorter than 1 byte', async () => {
      rawAssetLock.transaction = Buffer.alloc(0);

      const result = await validateAssetLockStructure(rawAssetLock);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.transaction');
      expect(error.keyword).to.equal('minItems');
    });

    it('should be not longer than 100 Kb', async () => {
      rawAssetLock.transaction = Buffer.alloc(100001);

      const result = await validateAssetLockStructure(rawAssetLock);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.transaction');
      expect(error.keyword).to.equal('maxItems');
    });

    it('should be valid', async () => {
      rawAssetLock.transaction = Buffer.alloc(100, 1);

      const result = await validateAssetLockStructure(rawAssetLock);

      expectValidationError(result, InvalidIdentityAssetLockTransactionError);

      const [error] = result.getErrors();

      expect(error.message).to.equal('Invalid asset lock transaction: Unknown special transaction type');
    });
  });

  describe('outputIndex', () => {
    it('should be present', async () => {
      delete rawAssetLock.outputIndex;

      const result = await validateAssetLockStructure(rawAssetLock);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('outputIndex');
    });

    it('should be an integer', async () => {
      rawAssetLock.outputIndex = 1.1;

      const result = await validateAssetLockStructure(rawAssetLock);

      expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.outputIndex');
      expect(error.keyword).to.equal('type');
    });

    it('should be not less than 0', async () => {
      rawAssetLock.outputIndex = -1;

      const result = await validateAssetLockStructure(rawAssetLock);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.outputIndex');
      expect(error.keyword).to.equal('minimum');
    });

    it('should point to specific output in transaction', async () => {
      rawAssetLock.outputIndex = 10;

      const result = await validateAssetLockStructure(rawAssetLock);

      expectValidationError(result, IdentityAssetLockTransactionOutputNotFoundError);

      const [error] = result.getErrors();

      expect(error.getOutputIndex()).to.equal(rawAssetLock.outputIndex);
    });

    it('should point to output with OR_RETURN', async () => {
      rawAssetLock.outputIndex = 1;

      const result = await validateAssetLockStructure(rawAssetLock);

      expectValidationError(result, InvalidIdentityAssetLockTransactionOutputError);

      const [error] = result.getErrors();

      expect(error.message).to.equal('Invalid asset lock transaction output: Output is not a valid standard OP_RETURN output');
    });

    it('should point to output with public key hash', async () => {
      rawAssetLock.outputIndex = 2;

      const result = await validateAssetLockStructure(rawAssetLock);

      expectValidationError(result, InvalidIdentityAssetLockTransactionOutputError);

      const [error] = result.getErrors();

      expect(error.message).to.equal('Invalid asset lock transaction output: Output has invalid public key hash');
    });
  });

  describe('proof', () => {
    it('should be present', async () => {
      delete rawAssetLock.proof;

      const result = await validateAssetLockStructure(rawAssetLock);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('proof');
    });

    it('should be an object', async () => {
      rawAssetLock.proof = 1;

      const result = await validateAssetLockStructure(rawAssetLock);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.proof');
      expect(error.keyword).to.equal('type');
    });

    describe('type', () => {
      it('should be present', async () => {
        delete rawAssetLock.proof.type;

        const result = await validateAssetLockStructure(rawAssetLock);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.proof');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('type');
      });

      it('should be equal to 0', async () => {
        rawAssetLock.proof.type = -1;

        const result = await validateAssetLockStructure(rawAssetLock);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.proof.type');
        expect(error.keyword).to.equal('enum');
      });
    });
  });

  it('should return invalid result if proof is not valid', async () => {
    const proofError = new ConsensusError('something');

    proofValidationFunctionMock.resolves(
      new ValidationResult([proofError]),
    );

    const result = await validateAssetLockStructure(rawAssetLock);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(proofError);
  });

  it('should return valid result with public key hash', async () => {
    proofValidationFunctionMock.resolves(
      new ValidationResult(),
    );

    const result = await validateAssetLockStructure(rawAssetLock);

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(result.getData()).to.be.instanceOf(Buffer);
    expect(result.getData()).to.have.lengthOf(20);

    const transaction = new Transaction(rawAssetLock.transaction);

    expect(proofValidationFunctionMock).to.be.calledOnce();

    const { args } = proofValidationFunctionMock.getCall(0);

    expect(args[0]).to.be.deep.equal(rawAssetLock);
    expect(args[1].toBuffer()).to.be.deep.equal(transaction.toBuffer());
  });
});
