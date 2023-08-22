const crypto = require('crypto');

const getIdentityFixture = require('../../../../lib/test/fixtures/getIdentityFixture');
const getBlsMock = require('../../../../lib/test/mocks/getBlsAdapterMock');

const {
  expectValidationError,
  expectJsonSchemaError,
} = require('../../../../lib/test/expect/expectError');

const { default: loadWasmDpp } = require('../../../../dist');

describe.skip('validatePublicKeysFactory', function main() {
  this.timeout(10000);

  let rawPublicKeys;
  let validatePublicKeys;
  let publicKeysValidator;

  let PublicKeysValidator;
  let IdentityPublicKey;

  let DuplicatedIdentityPublicKeyError;
  let DuplicatedIdentityPublicKeyIdError;
  let InvalidIdentityPublicKeyDataError;
  let InvalidIdentityPublicKeySecurityLevelError;

  beforeEach(async () => {
    ({ publicKeys: rawPublicKeys } = (await getIdentityFixture()).toObject());

    ({
      PublicKeysValidator, IdentityPublicKey,
      InvalidIdentityPublicKeyDataError,
      DuplicatedIdentityPublicKeyError, DuplicatedIdentityPublicKeyIdError,
      InvalidIdentityPublicKeyDataError, InvalidIdentityPublicKeySecurityLevelError,
    } = await loadWasmDpp());

    const blsAdapter = await getBlsMock();

    publicKeysValidator = new PublicKeysValidator(blsAdapter);

    validatePublicKeys = (keys) => publicKeysValidator.validateKeys(keys);
  });

  describe('id', () => {
    it('should be present', async () => {
      delete rawPublicKeys[1].id;

      const result = publicKeysValidator.validateKeys(rawPublicKeys);
      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('id');
    });

    it('should be a number', async () => {
      rawPublicKeys[1].id = 'string';

      const result = validatePublicKeys(rawPublicKeys);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/id');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be an integer', async () => {
      rawPublicKeys[1].id = 1.1;

      const result = validatePublicKeys(rawPublicKeys);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/id');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be greater or equal to one', async () => {
      rawPublicKeys[1].id = -1;

      const result = validatePublicKeys(rawPublicKeys);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/id');
      expect(error.getKeyword()).to.equal('minimum');
    });
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawPublicKeys[1].type;

      const result = validatePublicKeys(rawPublicKeys);

      await expectJsonSchemaError(result, 4);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/data');
      expect(error.getKeyword()).to.equal('minItems');
    });

    it('should be a number', async () => {
      rawPublicKeys[1].type = 'string';

      const result = validatePublicKeys(rawPublicKeys);

      await expectJsonSchemaError(result, 2);

      const [typeError, enumError] = result.getErrors();

      expect(typeError.getInstancePath()).to.equal('/type');
      expect(typeError.getKeyword()).to.equal('type');

      expect(enumError.getInstancePath()).to.equal('/type');
      expect(enumError.getKeyword()).to.equal('enum');
    });
  });

  describe('data', () => {
    it('should be present', async () => {
      delete rawPublicKeys[1].data;

      const result = validatePublicKeys(rawPublicKeys);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('data');
    });

    it('should be a byte array', async () => {
      rawPublicKeys[1].data = new Array(33).fill('string');

      const result = validatePublicKeys(rawPublicKeys);

      await expectJsonSchemaError(result, 33);

      const [error, byteArrayError] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/data/0');
      expect(error.getKeyword()).to.equal('type');

      expect(byteArrayError.getInstancePath()).to.equal('/data/1');
      expect(byteArrayError.getKeyword()).to.equal('type');
    });

    describe('ECDSA_SECP256K1', () => {
      it('should be no less than 33 bytes', async () => {
        rawPublicKeys[1].data = Buffer.alloc(32);

        const result = validatePublicKeys(rawPublicKeys);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/data');
        expect(error.getKeyword()).to.equal('minItems');
      });

      it('should be no longer than 33 bytes', async () => {
        rawPublicKeys[1].data = Buffer.alloc(34);

        const result = validatePublicKeys(rawPublicKeys);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/data');
        expect(error.getKeyword()).to.equal('maxItems');
      });
    });

    describe('BLS12_381', () => {
      it('should be no less than 48 bytes', async () => {
        rawPublicKeys[1].data = Buffer.alloc(47);
        rawPublicKeys[1].type = 1;

        const result = validatePublicKeys(rawPublicKeys);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/data');
        expect(error.getKeyword()).to.equal('minItems');
      });

      it('should be no longer than 48 bytes', async () => {
        rawPublicKeys[1].data = Buffer.alloc(49);
        rawPublicKeys[1].type = 1;

        const result = validatePublicKeys(rawPublicKeys);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/data');
        expect(error.getKeyword()).to.equal('maxItems');
      });
    });

    describe('ECDSA_HASH160', () => {
      it('should be no less than 20 bytes', async () => {
        rawPublicKeys[1].data = Buffer.alloc(19);
        rawPublicKeys[1].type = 2;

        const result = validatePublicKeys(rawPublicKeys);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/data');
        expect(error.getKeyword()).to.equal('minItems');
      });

      it('should be no longer than 20 bytes', async () => {
        rawPublicKeys[1].data = Buffer.alloc(21);
        rawPublicKeys[1].type = 2;

        const result = validatePublicKeys(rawPublicKeys);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/data');
        expect(error.getKeyword()).to.equal('maxItems');
      });
    });

    describe('BIP13_SCRIPT_HASH', () => {
      it('should be no less than 20 bytes', async () => {
        rawPublicKeys[1].data = Buffer.alloc(19);
        rawPublicKeys[1].type = 3;

        const result = validatePublicKeys(rawPublicKeys);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/data');
        expect(error.getKeyword()).to.equal('minItems');
      });

      it('should be no longer than 20 bytes', async () => {
        rawPublicKeys[1].data = Buffer.alloc(21);
        rawPublicKeys[1].type = 3;

        const result = validatePublicKeys(rawPublicKeys);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/data');
        expect(error.getKeyword()).to.equal('maxItems');
      });
    });
  });

  it('should return invalid result if there are duplicate key ids', async () => {
    rawPublicKeys[1].id = rawPublicKeys[0].id;

    const result = validatePublicKeys(rawPublicKeys);

    await expectValidationError(result, DuplicatedIdentityPublicKeyIdError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1030);
    expect(error.getDuplicatedIds()).to.deep.equal([rawPublicKeys[1].id]);
  });

  it('should return invalid result if there are duplicate keys', async () => {
    rawPublicKeys[1].data = rawPublicKeys[0].data;

    const result = validatePublicKeys(rawPublicKeys);

    await expectValidationError(result, DuplicatedIdentityPublicKeyError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1029);
    expect(error.getDuplicatedPublicKeysIds()).to.deep.equal([rawPublicKeys[1].id]);
  });

  it('should return invalid result if key data is not a valid DER', async () => {
    rawPublicKeys[1].data = Buffer.alloc(33);

    const result = validatePublicKeys(rawPublicKeys);

    await expectValidationError(result, InvalidIdentityPublicKeyDataError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1040);
    expect(error.getPublicKeyId()).to.deep.equal(rawPublicKeys[1].id);
    expect(error.getValidationError()).to.equal('Key secp256k1 error: malformed public key');
  });

  it('should return invalid result if key has an invalid combination of purpose and security level', async () => {
    rawPublicKeys[1].purpose = IdentityPublicKey.PURPOSES.ENCRYPTION;
    rawPublicKeys[1].securityLevel = IdentityPublicKey.SECURITY_LEVELS.MASTER;

    const result = validatePublicKeys(rawPublicKeys);

    await expectValidationError(result, InvalidIdentityPublicKeySecurityLevelError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1047);
    expect(error.getPublicKeyId()).to.deep.equal(rawPublicKeys[1].id);
    expect(error.getPublicKeySecurityLevel()).to.be.equal(rawPublicKeys[1].securityLevel);
    expect(error.getPublicKeyPurpose()).to.equal(rawPublicKeys[1].purpose);
  });

  it('should pass valid public keys', () => {
    const result = validatePublicKeys(rawPublicKeys);

    expect(result.isValid()).to.be.true();
  });

  it('should pass valid BLS12_381 public key', () => {
    rawPublicKeys = [{
      id: 0,
      type: IdentityPublicKey.TYPES.BLS12_381,
      purpose: 0,
      securityLevel: 0,
      readOnly: true,
      data: Buffer.from('928f4cdf8d7e05527f26e43348764832d55f5cca0107097fe9af57383f46da612428f617d810186419344f76a14efe96', 'hex'),
    }];

    const result = validatePublicKeys(rawPublicKeys);

    expect(result.isValid()).to.be.true();
  });

  it('should pass valid ECDSA_HASH160 public key', () => {
    rawPublicKeys = [{
      id: 0,
      type: IdentityPublicKey.TYPES.ECDSA_HASH160,
      purpose: 0,
      securityLevel: 0,
      readOnly: true,
      data: Buffer.from('6086389d3fa4773aa950b8de18c5bd6d8f2b73bc', 'hex'),
    }];

    const result = validatePublicKeys(rawPublicKeys);

    expect(result.isValid()).to.be.true();
  });

  it('should return invalid result if BLS12_381 public key is invalid', async () => {
    rawPublicKeys = [{
      id: 0,
      type: IdentityPublicKey.TYPES.BLS12_381,
      purpose: 0,
      securityLevel: 0,
      readOnly: true,
      data: Buffer.from('11fac99ca2c8f39c286717c213e190aba4b7af76db320ec43f479b7d9a2012313a0ae59ca576edf801444bc694686694', 'hex'),
    }];

    const result = publicKeysValidator.validateKeys(rawPublicKeys);

    await expectValidationError(result, InvalidIdentityPublicKeyDataError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1040);
    expect(error.getPublicKeyId()).to.deep.equal(rawPublicKeys[0].id);
    expect(error.getValidationError()).to.equal('Invalid public key');
  });

  describe('Identity Schema', () => {
    beforeEach(() => {
      rawPublicKeys[0].disabledAt = new Date().getTime();
    });

    describe('disabledAt', () => {
      it('should be an integer');

      it('should be greater than 0');
    });
  });

  describe('State Transition Schema', () => {
    beforeEach(() => {
      validatePublicKeys = (keys) => publicKeysValidator.validateKeysInStateTransition(keys);

      rawPublicKeys.forEach((rawPublicKey) => {
        // eslint-disable-next-line no-param-reassign
        rawPublicKey.signature = crypto.randomBytes(65);
      });
    });

    describe('signature', () => {
      it('should be present', async () => {
        delete rawPublicKeys[0].signature;

        const result = validatePublicKeys(rawPublicKeys);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('signature');
      });

      it('should be a byte array', async () => {
        rawPublicKeys[0].signature = new Array(65).fill('string');

        const result = validatePublicKeys(rawPublicKeys);

        await expectJsonSchemaError(result, 65);

        const [error] = result.getErrors();

        expect(error.getKeyword()).to.equal('type');
        expect(error.getInstancePath()).to.equal('/signature/0');
      });

      it('should be not shorter than 65 bytes', async () => {
        rawPublicKeys[0].signature = Buffer.alloc(64);

        const result = validatePublicKeys(rawPublicKeys);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/signature');
        expect(error.getKeyword()).to.equal('minItems');
      });

      it('should be not longer than 65 bytes', async () => {
        rawPublicKeys[0].signature = Buffer.alloc(66);

        const result = validatePublicKeys(rawPublicKeys);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/signature');
        expect(error.getKeyword()).to.equal('maxItems');
      });
    });
  });
});
