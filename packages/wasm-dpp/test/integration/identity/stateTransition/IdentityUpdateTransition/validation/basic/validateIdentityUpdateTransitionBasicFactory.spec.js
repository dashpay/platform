const { PrivateKey } = require('@dashevo/dashcore-lib');
const getIdentityUpdateTransitionFixture = require('../../../../../../../lib/test/fixtures/getIdentityUpdateTransitionFixture');

const { expectJsonSchemaError, expectValidationError } = require('../../../../../../../lib/test/expect/expectError');
const { default: loadWasmDpp } = require('../../../../../../../dist');
const getBlsAdapterMock = require('../../../../../../../lib/test/mocks/getBlsAdapterMock');

describe.skip('validateIdentityUpdateTransitionBasicFactory', () => {
  let validateIdentityUpdateTransitionBasic;
  let rawStateTransition;
  let stateTransition;
  let publicKeyToAdd;

  let IdentityPublicKey;
  let IdentityPublicKeyWithWitness;
  let UnsupportedProtocolVersionError;
  let InvalidIdentityKeySignatureError;
  let DuplicatedIdentityPublicKeyIdStateError;
  let IdentityUpdateTransitionBasicValidator;

  before(async () => {
    ({
      UnsupportedProtocolVersionError,
      InvalidIdentityKeySignatureError,
      DuplicatedIdentityPublicKeyIdStateError,
      IdentityPublicKey,
      IdentityPublicKeyWithWitness,
      IdentityUpdateTransitionBasicValidator,
    } = await loadWasmDpp());
  });

  beforeEach(async () => {
    const blsAdapter = await getBlsAdapterMock();

    const validator = new IdentityUpdateTransitionBasicValidator(blsAdapter);
    validateIdentityUpdateTransitionBasic = (st) => validator.validate(st);

    stateTransition = await getIdentityUpdateTransitionFixture();

    const privateKey = new PrivateKey('9b67f852093bc61cea0eeca38599dbfba0de28574d2ed9b99d10d33dc1bde7b2');
    const publicKey = privateKey.toPublicKey().toBuffer();

    const identityPublicKey = new IdentityPublicKey({
      id: 1,
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      data: publicKey,
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
      readOnly: false,
    });

    let identityPublicKeyCreateTransition = new IdentityPublicKeyWithWitness({
      ...identityPublicKey.toObject(),
      signature: Buffer.alloc(0),
    });

    stateTransition.setPublicKeysToAdd([identityPublicKeyCreateTransition]);

    await stateTransition.sign(identityPublicKey, privateKey.toBuffer());

    [identityPublicKeyCreateTransition] = stateTransition.getPublicKeysToAdd();
    identityPublicKeyCreateTransition.setSignature(stateTransition.getSignature());
    stateTransition.setPublicKeysToAdd([identityPublicKeyCreateTransition]);

    rawStateTransition = stateTransition.toObject();

    publicKeyToAdd = {
      id: 0,
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      data: Buffer.from('AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di', 'base64'),
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
      readOnly: false,
      signature: Buffer.alloc(0),
    };
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {
      delete rawStateTransition.protocolVersion;

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('protocolVersion');
    });

    it('should be integer', async () => {
      rawStateTransition.protocolVersion = '1';

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/protocolVersion');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be valid', async () => {
      rawStateTransition.protocolVersion = 100;

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      await expectValidationError(result, UnsupportedProtocolVersionError);
    });
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawStateTransition.type;

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('type');
    });

    it('should be equal to 5', async () => {
      rawStateTransition.type = 666;

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/type');
      expect(error.getKeyword()).to.equal('const');
      expect(error.getParams().allowedValue).to.equal(5);
    });
  });

  describe('identityId', () => {
    it('should be present', async () => {
      delete rawStateTransition.identityId;

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('identityId');
    });

    it('should be a byte array', async () => {
      rawStateTransition.identityId = new Array(32).fill('string');

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      await expectJsonSchemaError(result, 32);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/identityId/0');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be no less than 32 bytes', async () => {
      rawStateTransition.identityId = Buffer.alloc(31);

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/identityId');
      expect(error.getKeyword()).to.equal('minItems');
    });

    it('should be no longer than 32 bytes', async () => {
      rawStateTransition.identityId = Buffer.alloc(33);

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
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

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('signature');
    });

    it('should be a byte array', async () => {
      rawStateTransition.signature = new Array(65).fill('string');

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result, 65);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/signature/0');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be not shorter than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(64);

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/signature');
      expect(error.getKeyword()).to.equal('minItems');
    });

    it('should be not longer than 96 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(97);

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/signature');
      expect(error.getKeyword()).to.equal('maxItems');
    });
  });

  describe('revision', () => {
    it('should be present', async () => {
      delete rawStateTransition.revision;

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('revision');
    });

    it('should be integer', async () => {
      rawStateTransition.revision = '1';

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/revision');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be greater or equal 0', async () => {
      rawStateTransition.revision = -1;

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('minimum');
      expect(error.getInstancePath()).to.equal('/revision');
    });
  });

  describe('addPublicKeys', async () => {
    beforeEach(() => {
      delete rawStateTransition.disablePublicKeys;
      delete rawStateTransition.publicKeysDisabledAt;
    });

    it('should return valid result', async () => {
      const privateKey = new PrivateKey();
      const publicKey = privateKey.toPublicKey();

      const identityPublicKeyCreateTransition = new IdentityPublicKeyWithWitness(
        publicKeyToAdd,
      );
      identityPublicKeyCreateTransition.setData(publicKey.toBuffer());

      stateTransition.setPublicKeysToAdd([identityPublicKeyCreateTransition]);
      await stateTransition.signByPrivateKey(
        privateKey.toBuffer(),
        identityPublicKeyCreateTransition.type,
      );
      identityPublicKeyCreateTransition.setSignature(stateTransition.getSignature());
      stateTransition.setPublicKeysToAdd([identityPublicKeyCreateTransition]);

      rawStateTransition = stateTransition.toObject();

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      expect(result.isValid()).to.be.true();
    });

    it('should not be empty', async () => {
      rawStateTransition.addPublicKeys = [];

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();
      expect(error.getKeyword()).to.equal('minItems');
      expect(error.getInstancePath()).to.equal('/addPublicKeys');
    });

    it('should not have more than 10 items', async () => {
      rawStateTransition.addPublicKeys = [];

      for (let i = 0; i <= 10; i++) {
        rawStateTransition.addPublicKeys.push({ ...publicKeyToAdd, id: i + 1 });
      }

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('maxItems');
      expect(error.getInstancePath()).to.equal('/addPublicKeys');
    });

    it('should be unique', async () => {
      rawStateTransition.addPublicKeys = [publicKeyToAdd, publicKeyToAdd];

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('uniqueItems');
      expect(error.getInstancePath()).to.equal('/addPublicKeys');
    });

    it('should be valid', async () => {
      const keyWithDupeId = { ...publicKeyToAdd, data: Buffer.alloc(33) };

      stateTransition.setPublicKeysToAdd([
        new IdentityPublicKeyWithWitness(publicKeyToAdd),
        new IdentityPublicKeyWithWitness(keyWithDupeId),
      ]);

      rawStateTransition = stateTransition.toObject();

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      await expectValidationError(result, DuplicatedIdentityPublicKeyIdStateError);
    });

    it('should have valid signatures', async () => {
      const invalidSignature = Buffer.alloc(65);
      rawStateTransition.signature = invalidSignature;
      rawStateTransition.addPublicKeys[0].signature = invalidSignature;

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      await expectValidationError(result, InvalidIdentityKeySignatureError);
    });
  });

  describe('disablePublicKeys', async () => {
    beforeEach(() => {
      delete rawStateTransition.addPublicKeys;
    });

    it('should be used only with publicKeysDisabledAt', async () => {
      delete rawStateTransition.publicKeysDisabledAt;

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('publicKeysDisabledAt');
    });

    it('should be valid', async () => {
      rawStateTransition.disablePublicKeys = [0];
      rawStateTransition.publicKeysDisabledAt = 0;

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      expect(result.isValid()).to.be.true();
    });

    it('should contain numbers >= 0', async () => {
      rawStateTransition.disablePublicKeys = [-1, 0];
      rawStateTransition.publicKeysDisabledAt = 0;

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/disablePublicKeys/0');
      expect(error.getKeyword()).to.equal('minimum');
    });

    it('should contain integers', async () => {
      rawStateTransition.publicKeysDisabledAt = 0;
      rawStateTransition.disablePublicKeys = [1.1];

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/disablePublicKeys/0');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should not have more than 10 items', async () => {
      rawStateTransition.publicKeysDisabledAt = 0;
      rawStateTransition.disablePublicKeys = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('maxItems');
      expect(error.getInstancePath()).to.equal('/disablePublicKeys');
    });

    it('should be unique', async () => {
      rawStateTransition.publicKeysDisabledAt = 0;
      rawStateTransition.disablePublicKeys = [0, 0];

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('uniqueItems');
      expect(error.getInstancePath()).to.equal('/disablePublicKeys');
    });
  });

  describe('publicKeysDisabledAt', async () => {
    it('should be used only with disablePublicKeys', async () => {
      delete rawStateTransition.disablePublicKeys;

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('disablePublicKeys');
    });

    it('should be integer', async () => {
      rawStateTransition.publicKeysDisabledAt = 1.1;
      rawStateTransition.disablePublicKeys = [0];

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/publicKeysDisabledAt');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be >= 0', async () => {
      rawStateTransition.publicKeysDisabledAt = -1;
      rawStateTransition.disablePublicKeys = [0];

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/publicKeysDisabledAt');
      expect(error.getKeyword()).to.equal('minimum');
    });
  });

  it('should return valid result', async () => {
    const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

    expect(result.isValid()).to.be.true();
  });

  it('should have either addPublicKeys or disablePublicKeys', async () => {
    delete rawStateTransition.disablePublicKeys;
    delete rawStateTransition.addPublicKeys;
    delete rawStateTransition.publicKeysDisabledAt;

    const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

    await expectJsonSchemaError(result, 1);

    const [error] = result.getErrors();

    expect(error.getKeyword()).to.equal('anyOf');
  });
});
