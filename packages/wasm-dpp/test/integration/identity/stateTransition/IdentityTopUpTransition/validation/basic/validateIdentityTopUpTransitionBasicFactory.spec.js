const createStateRepositoryMock = require('../../../../../../../lib/test/mocks/createStateRepositoryMock');
const getIdentityTopUpTransitionFixture = require('../../../../../../../lib/test/fixtures/getIdentityTopUpTransitionFixture');

const {
  expectJsonSchemaError,
  expectValidationError,
} = require('../../../../../../../lib/test/expect/expectError');
const { default: loadWasmDpp } = require('../../../../../../../dist');

describe.skip('validateIdentityTopUpTransitionBasicFactory', () => {
  let rawStateTransition;
  let stateTransition;

  let stateRepositoryMock;
  let executionContext;

  let validateIdentityTopUpTransitionBasic;

  let StateTransitionExecutionContext;
  let IdentityPublicKey;
  let UnsupportedProtocolVersionError;
  let InvalidInstantAssetLockProofSignatureError;
  let IdentityTopUpTransitionBasicValidator;

  before(async () => {
    ({
      IdentityTopUpTransitionBasicValidator,
      StateTransitionExecutionContext,
      IdentityPublicKey,
      UnsupportedProtocolVersionError,
      InvalidInstantAssetLockProofSignatureError,
    } = await loadWasmDpp());
  });

  beforeEach(async function beforeEach() {
    stateRepositoryMock = createStateRepositoryMock(this.sinon);
    stateRepositoryMock.verifyInstantLock.resolves(true);
    stateRepositoryMock.isAssetLockTransactionOutPointAlreadyUsed.resolves(false);

    executionContext = new StateTransitionExecutionContext();

    const validator = new IdentityTopUpTransitionBasicValidator(stateRepositoryMock);
    validateIdentityTopUpTransitionBasic = (st, context) => validator.validate(
      st,
      context,
    );

    stateTransition = await getIdentityTopUpTransitionFixture();

    const privateKey = '9b67f852093bc61cea0eeca38599dbfba0de28574d2ed9b99d10d33dc1bde7b2';

    await stateTransition.signByPrivateKey(
      Buffer.from(privateKey, 'hex'),
      IdentityPublicKey.TYPES.ECDSA_SECP256K1,
    );

    rawStateTransition = stateTransition.toObject();
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {
      delete rawStateTransition.protocolVersion;

      const result = await validateIdentityTopUpTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('protocolVersion');
    });

    it('should be an integer', async () => {
      rawStateTransition.protocolVersion = '1';

      const result = await validateIdentityTopUpTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/protocolVersion');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be valid', async () => {
      rawStateTransition.protocolVersion = 1000;

      const result = await validateIdentityTopUpTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectValidationError(result, UnsupportedProtocolVersionError);
    });
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawStateTransition.type;

      const result = await validateIdentityTopUpTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('type');
    });

    it('should be equal to 3', async () => {
      rawStateTransition.type = 666;

      const result = await validateIdentityTopUpTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/type');
      expect(error.getKeyword()).to.equal('const');
      expect(error.getParams().allowedValue).to.equal(3);
    });
  });

  describe('assetLockProof', () => {
    it('should be present', async () => {
      delete rawStateTransition.assetLockProof;

      const result = await validateIdentityTopUpTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getParams().missingProperty).to.equal('assetLockProof');
      expect(error.getKeyword()).to.equal('required');
    });

    it('should be an object', async () => {
      rawStateTransition.assetLockProof = 1;

      const result = await validateIdentityTopUpTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/assetLockProof');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be valid', async () => {
      stateRepositoryMock.verifyInstantLock.resolves(false);

      const result = await validateIdentityTopUpTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectValidationError(result);

      const [error] = result.getErrors();

      expect(error).to.be.instanceOf(InvalidInstantAssetLockProofSignatureError);
    });
  });

  describe('identityId', () => {
    it('should be present', async () => {
      delete rawStateTransition.identityId;

      const result = await validateIdentityTopUpTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('identityId');
    });

    it('should be a byte array', async () => {
      rawStateTransition.identityId = new Array(32).fill('string');

      const result = await validateIdentityTopUpTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result, 32);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/identityId/0');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be no less than 32 bytes', async () => {
      rawStateTransition.identityId = Buffer.alloc(31);

      const result = await validateIdentityTopUpTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/identityId');
      expect(error.getKeyword()).to.equal('minItems');
    });

    it('should be no longer than 32 bytes', async () => {
      rawStateTransition.identityId = Buffer.alloc(33);

      const result = await validateIdentityTopUpTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/identityId');
      expect(error.getKeyword()).to.equal('maxItems');
    });
  });

  describe('signature', () => {
    it('should be present', async () => {
      delete rawStateTransition.signature;

      const result = await validateIdentityTopUpTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('signature');
    });

    it('should be a byte array', async () => {
      rawStateTransition.signature = new Array(65).fill('string');

      const result = await validateIdentityTopUpTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result, 65);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/signature/0');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be not shorter than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(64);

      const result = await validateIdentityTopUpTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/signature');
      expect(error.getKeyword()).to.equal('minItems');
    });

    it('should be not longer than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(66);

      const result = await validateIdentityTopUpTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/signature');
      expect(error.getKeyword()).to.equal('maxItems');
    });
  });

  it('should return valid result', async () => {
    const result = await validateIdentityTopUpTransitionBasic(
      rawStateTransition,
      executionContext,
    );

    expect(result.isValid()).to.be.true();
  });
});
