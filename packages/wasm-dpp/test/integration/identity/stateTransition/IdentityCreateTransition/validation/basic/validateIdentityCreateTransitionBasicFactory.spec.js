const { PrivateKey } = require('@dashevo/dashcore-lib');

const getIdentityCreateTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityCreateTransitionFixture');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const { expectValidationError, expectJsonSchemaError } = require('../../../../../../../lib/test/expect/expectError');
const { default: loadWasmDpp } = require('../../../../../../../dist');
const getBlsAdapterMock = require('../../../../../../../lib/test/mocks/getBlsAdapterMock');

describe('validateIdentityCreateTransitionBasicFactory', () => {
  let rawStateTransition;
  let stateTransition;

  let stateRepositoryMock;
  let mockIdentityPublicKeyCreateTransition;
  let executionContext;

  let validateIdentityCreateTransitionBasic;

  let StateTransitionExecutionContext;
  let IdentityCreateTransition;
  let IdentityPublicKey;
  let UnsupportedProtocolVersionError;
  let InvalidInstantAssetLockProofSignatureError;
  let InvalidIdentityPublicKeySecurityLevelError;
  let InvalidIdentityPublicKeyDataError;
  let InvalidIdentityKeySignatureError;
  let IdentityCreateTransitionBasicValidator;
  let IdentityPublicKeyCreateTransition;

  before(async () => {
    ({
      IdentityCreateTransition,
      StateTransitionExecutionContext,
      UnsupportedProtocolVersionError,
      InvalidInstantAssetLockProofSignatureError,
      InvalidIdentityPublicKeySecurityLevelError,
      InvalidIdentityPublicKeyDataError,
      InvalidIdentityKeySignatureError,
      IdentityPublicKeyCreateTransition,
      IdentityCreateTransitionBasicValidator,
      IdentityPublicKey,
    } = await loadWasmDpp());

    mockIdentityPublicKeyCreateTransition = (publicKey, opts = {}) => (
      new IdentityPublicKeyCreateTransition({
        id: 0,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: publicKey,
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
        readOnly: false,
        signature: Buffer.alloc(0),
        ...opts,
      })
    );
  });

  beforeEach(async function () {
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.verifyInstantLock.resolves(true);
    stateRepositoryMock.isAssetLockTransactionOutPointAlreadyUsed.resolves(false);

    executionContext = new StateTransitionExecutionContext();

    const blsAdapter = await getBlsAdapterMock();

    const validator = new IdentityCreateTransitionBasicValidator(stateRepositoryMock, blsAdapter);
    validateIdentityCreateTransitionBasic = (st, context) => validator.validate(st, context);

    const stateTransitionJS = getIdentityCreateTransitionFixture();
    stateTransition = new IdentityCreateTransition(stateTransitionJS.toObject());

    const privateKey = new PrivateKey('9b67f852093bc61cea0eeca38599dbfba0de28574d2ed9b99d10d33dc1bde7b2');
    const publicKey = privateKey.toPublicKey();

    const identityPublicKey = mockIdentityPublicKeyCreateTransition(publicKey.toBuffer());

    stateTransition.setPublicKeys([identityPublicKey]);

    // Sign a new public key

    await stateTransition.signByPrivateKey(
      privateKey.toBuffer(),
      identityPublicKey.type,
    );

    const publicKeySignature = stateTransition.getSignature();
    identityPublicKey.setSignature(publicKeySignature);

    stateTransition.setPublicKeys([identityPublicKey]);

    rawStateTransition = stateTransition.toObject();
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {
      delete rawStateTransition.protocolVersion;

      const result = await validateIdentityCreateTransitionBasic(
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

      const result = await validateIdentityCreateTransitionBasic(
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

      const result = await validateIdentityCreateTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectValidationError(result, UnsupportedProtocolVersionError);
    });
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawStateTransition.type;

      const result = await validateIdentityCreateTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('type');
    });

    it('should be equal to 2', async () => {
      rawStateTransition.type = 666;

      const result = await validateIdentityCreateTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/type');
      expect(error.getKeyword()).to.equal('const');
      expect(error.getParams().allowedValue).to.equal(2);
    });
  });

  describe('assetLockProof', () => {
    it('should be present', async () => {
      delete rawStateTransition.assetLockProof;

      const result = await validateIdentityCreateTransitionBasic(
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

      const result = await validateIdentityCreateTransitionBasic(
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

      const result = await validateIdentityCreateTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectValidationError(result);

      const [error] = result.getErrors();

      expect(error).to.be.instanceOf(InvalidInstantAssetLockProofSignatureError);
    });
  });

  describe('publicKeys', () => {
    it('should be present', async () => {
      rawStateTransition.publicKeys = undefined;

      const result = await validateIdentityCreateTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getParams().missingProperty).to.equal('publicKeys');
      expect(error.getKeyword()).to.equal('required');
    });

    it('should not be empty', async () => {
      rawStateTransition.publicKeys = [];

      const result = await validateIdentityCreateTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('minItems');
      expect(error.getInstancePath()).to.equal('/publicKeys');
    });

    it('should not have more than 10 items', async () => {
      const [key] = rawStateTransition.publicKeys;

      for (let i = 0; i < 10; i++) {
        rawStateTransition.publicKeys.push({ ...key, id: i + 1 });
      }

      const result = await validateIdentityCreateTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('maxItems');
      expect(error.getInstancePath()).to.equal('/publicKeys');
    });

    it('should be unique', async () => {
      rawStateTransition.publicKeys.push(rawStateTransition.publicKeys[0]);

      const result = await validateIdentityCreateTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('uniqueItems');
      expect(error.getInstancePath()).to.equal('/publicKeys');
    });

    it('should be valid', async () => {
      const privateKey = new PrivateKey();

      // Mess up public key
      const identityPublicKey = mockIdentityPublicKeyCreateTransition(Buffer.alloc(33));

      stateTransition.setPublicKeys([identityPublicKey]);
      await stateTransition.signByPrivateKey(
        privateKey.toBuffer(),
        identityPublicKey.type,
      );
      identityPublicKey.setSignature(stateTransition.getSignature());
      stateTransition.setPublicKeys([identityPublicKey]);

      rawStateTransition = stateTransition.toObject();

      const result = await validateIdentityCreateTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectValidationError(result, InvalidIdentityPublicKeyDataError);
    });

    it('should have at least 1 master key', async () => {
      const privateKey = new PrivateKey();

      // Mess up public key's purpose
      const identityPublicKey = mockIdentityPublicKeyCreateTransition(
        privateKey.toPublicKey().toBuffer(),
      );

      identityPublicKey.setPurpose(2);

      stateTransition.setPublicKeys([identityPublicKey]);
      await stateTransition.signByPrivateKey(
        privateKey.toBuffer(),
        identityPublicKey.type,
      );
      identityPublicKey.setSignature(stateTransition.getSignature());
      stateTransition.setPublicKeys([identityPublicKey]);

      rawStateTransition = stateTransition.toObject();

      const result = await validateIdentityCreateTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectValidationError(result, InvalidIdentityPublicKeySecurityLevelError);
    });

    it('should have valid signatures', async () => {
      const invalidSignature = Buffer.alloc(65);
      rawStateTransition.signature = invalidSignature;
      rawStateTransition.publicKeys[0].signature = invalidSignature;

      const result = await validateIdentityCreateTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectValidationError(result, InvalidIdentityKeySignatureError);
    });
  });

  describe('signature', () => {
    it('should be present', async () => {
      delete rawStateTransition.signature;

      const result = await validateIdentityCreateTransitionBasic(
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

      const result = await validateIdentityCreateTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result, 65);

      const firstError = result.getErrors()[0];
      const lastError = result.getErrors()[64];

      expect(firstError.getInstancePath()).to.equal('/signature/0');
      expect(firstError.getKeyword()).to.equal('type');

      expect(lastError.getInstancePath()).to.equal('/signature/64');
      expect(lastError.getKeyword()).to.equal('type');
    });

    it('should be not shorter than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(64);

      const result = await validateIdentityCreateTransitionBasic(
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

      const result = await validateIdentityCreateTransitionBasic(
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
    const result = await validateIdentityCreateTransitionBasic(
      rawStateTransition,
      executionContext,
    );

    expect(result.isValid()).to.be.true();
  });
});
