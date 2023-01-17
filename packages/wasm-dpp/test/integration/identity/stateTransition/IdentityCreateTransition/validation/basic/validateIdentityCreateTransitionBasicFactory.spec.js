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
  let mockIdentityPublicKey;

  let validateIdentityCreateTransitionBasic;

  let StateTransitionExecutionContext;
  let IdentityCreateTransition;
  let IdentityPublicKey;
  let UnsupportedProtocolVersionError;
  let InvalidInstantAssetLockProofSignatureError;
  let InvalidIdentityPublicKeySecurityLevelError;
  let InvalidIdentityPublicKeyDataError;
  let InvalidIdentityKeySignatureError;
  let validateIdentityCreateTransitionBasicDPP;

  before(async () => {
    ({
      validateIdentityCreateTransitionBasic: validateIdentityCreateTransitionBasicDPP,
      IdentityCreateTransition,
      StateTransitionExecutionContext,
      UnsupportedProtocolVersionError,
      InvalidInstantAssetLockProofSignatureError,
      InvalidIdentityPublicKeySecurityLevelError,
      InvalidIdentityPublicKeyDataError,
      InvalidIdentityKeySignatureError,
      IdentityPublicKey,
    } = await loadWasmDpp());

    mockIdentityPublicKey = (publicKey, opts = {}) => new IdentityPublicKey({
      id: 0,
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      data: publicKey,
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
      readOnly: false,
      ...opts,
    });
  });

  beforeEach(async function () {
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.verifyInstantLock.returns(true);

    const executionContext = new StateTransitionExecutionContext();

    const blsAdapter = await getBlsAdapterMock();

    validateIdentityCreateTransitionBasic = (st) => validateIdentityCreateTransitionBasicDPP(
      stateRepositoryMock,
      st,
      blsAdapter,
      executionContext,
    );

    const stateTransitionJS = getIdentityCreateTransitionFixture();
    stateTransition = new IdentityCreateTransition(stateTransitionJS.toObject());

    const privateKey = new PrivateKey();
    const publicKey = privateKey.toPublicKey();

    const identityPublicKey = mockIdentityPublicKey(publicKey.toBuffer());

    stateTransition.setPublicKeys([identityPublicKey]);

    await stateTransition.signByPrivateKey(
      privateKey.toBuffer(),
      identityPublicKey.type,
    );

    const signature = stateTransition.getSignature();
    identityPublicKey.setSignature(signature);

    stateTransition.setPublicKeys([identityPublicKey]);

    rawStateTransition = stateTransition.toObject();
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {
      delete rawStateTransition.protocolVersion;

      const result = await validateIdentityCreateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('protocolVersion');
    });

    it('should be an integer', async () => {
      rawStateTransition.protocolVersion = '1';

      const result = await validateIdentityCreateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/protocolVersion');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be valid', async () => {
      rawStateTransition.protocolVersion = 1000;

      const result = await validateIdentityCreateTransitionBasic(rawStateTransition);

      await expectValidationError(result, UnsupportedProtocolVersionError);
    });
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawStateTransition.type;

      const result = await validateIdentityCreateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('type');
    });

    it('should be equal to 2', async () => {
      rawStateTransition.type = 666;

      const result = await validateIdentityCreateTransitionBasic(rawStateTransition);

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
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getParams().missingProperty).to.equal('assetLockProof');
      expect(error.getKeyword()).to.equal('required');
    });

    it('should be an object', async () => {
      rawStateTransition.assetLockProof = 1;

      const result = await validateIdentityCreateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/assetLockProof');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be valid', async () => {
      stateRepositoryMock.verifyInstantLock.returns(false);

      const result = await validateIdentityCreateTransitionBasic(rawStateTransition);

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
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('uniqueItems');
      expect(error.getInstancePath()).to.equal('/publicKeys');
    });

    it('should be valid', async () => {
      const privateKey = new PrivateKey();

      // Mess up public key
      const identityPublicKey = mockIdentityPublicKey(Buffer.alloc(33));

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
      );

      await expectValidationError(result, InvalidIdentityPublicKeyDataError);
    });

    it('should have at least 1 master key', async () => {
      const privateKey = new PrivateKey();

      // Mess up public key's purpose
      const identityPublicKey = mockIdentityPublicKey(privateKey.toPublicKey().toBuffer());
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
      );

      await expectValidationError(result, InvalidIdentityPublicKeySecurityLevelError);
    });

    it('should have valid signatures', async () => {
      const invalidSignature = Buffer.alloc(65);
      rawStateTransition.signature = invalidSignature;
      rawStateTransition.publicKeys[0].signature = invalidSignature;

      const result = await validateIdentityCreateTransitionBasic(
        rawStateTransition,
      );

      await expectValidationError(result, InvalidIdentityKeySignatureError);
    });
  });

  describe('signature', () => {
    it('should be present', async () => {
      delete rawStateTransition.signature;

      const result = await validateIdentityCreateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('signature');
    });

    it('should be a byte array', async () => {
      rawStateTransition.signature = new Array(65).fill('string');

      const result = await validateIdentityCreateTransitionBasic(rawStateTransition);

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

      const result = await validateIdentityCreateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/signature');
      expect(error.getKeyword()).to.equal('minItems');
    });

    it('should be not longer than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(66);

      const result = await validateIdentityCreateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/signature');
      expect(error.getKeyword()).to.equal('maxItems');
    });
  });

  it('should return valid result', async () => {
    const result = await validateIdentityCreateTransitionBasic(rawStateTransition);

    expect(result.isValid()).to.be.true();
  });
});
