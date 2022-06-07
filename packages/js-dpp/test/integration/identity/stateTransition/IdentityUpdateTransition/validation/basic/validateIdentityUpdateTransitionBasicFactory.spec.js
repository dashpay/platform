const { getRE2Class } = require('@dashevo/wasm-re2');
const { PrivateKey } = require('@dashevo/dashcore-lib');
const validateIdentityUpdateTransitionBasicFactory = require(
  '../../../../../../../lib/identity/stateTransition/IdentityUpdateTransition/validation/basic/validateIdentityUpdateTransitionBasicFactory',
);
const JsonSchemaValidator = require('../../../../../../../lib/validation/JsonSchemaValidator');
const createAjv = require('../../../../../../../lib/ajv/createAjv');
const ValidationResult = require('../../../../../../../lib/validation/ValidationResult');
const IdentityPublicKey = require('../../../../../../../lib/identity/IdentityPublicKey');
const getIdentityUpdateTransitionFixture = require('../../../../../../../lib/test/fixtures/getIdentityUpdateTransitionFixture');
const { expectJsonSchemaError, expectValidationError } = require('../../../../../../../lib/test/expect/expectError');
const SomeConsensusError = require('../../../../../../../lib/test/mocks/SomeConsensusError');

describe('validateIdentityUpdateTransitionBasicFactory', () => {
  let validateIdentityUpdateTransitionBasic;
  let validateProtocolVersionMock;
  let validatePublicKeysMock;
  let rawStateTransition;
  let stateTransition;
  let publicKeyToAdd;
  let validatePublicKeySignaturesMock;

  beforeEach(async function beforeEach() {
    const RE2 = await getRE2Class();
    const ajv = createAjv(RE2);
    const jsonSchemaValidator = new JsonSchemaValidator(ajv);

    validateProtocolVersionMock = this.sinonSandbox.stub().returns(new ValidationResult());

    validatePublicKeysMock = this.sinonSandbox.stub()
      .returns(new ValidationResult());

    validatePublicKeySignaturesMock = this.sinonSandbox.stub()
      .returns(new ValidationResult());

    validateIdentityUpdateTransitionBasic = validateIdentityUpdateTransitionBasicFactory(
      jsonSchemaValidator,
      validateProtocolVersionMock,
      validatePublicKeysMock,
      validatePublicKeySignaturesMock,
    );

    stateTransition = getIdentityUpdateTransitionFixture();

    const privateKeyModel = new PrivateKey();
    const privateKeyHex = privateKeyModel.toBuffer().toString('hex');
    const publicKey = privateKeyModel.toPublicKey().toBuffer();

    const identityPublicKey = new IdentityPublicKey()
      .setId(1)
      .setType(IdentityPublicKey.TYPES.ECDSA_SECP256K1)
      .setData(publicKey)
      .setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.MASTER)
      .setPurpose(IdentityPublicKey.PURPOSES.AUTHENTICATION);

    await stateTransition.sign(identityPublicKey, privateKeyHex);

    rawStateTransition = stateTransition.toObject();

    publicKeyToAdd = {
      id: 0,
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      data: Buffer.from('AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di', 'base64'),
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
      readOnly: false,
    };
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {
      delete rawStateTransition.protocolVersion;

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('protocolVersion');
    });

    it('should be integer', async () => {
      rawStateTransition.protocolVersion = '1';

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/protocolVersion');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be valid', async () => {
      rawStateTransition.protocolVersion = -1;
      const protocolVersionError = new SomeConsensusError('test');
      const protocolVersionResult = new ValidationResult([
        protocolVersionError,
      ]);

      validateProtocolVersionMock.returns(protocolVersionResult);

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      expectValidationError(result, SomeConsensusError);

      const [error] = result.getErrors();

      expect(error).to.equal(protocolVersionError);

      expect(validateProtocolVersionMock).to.be.calledOnceWith(
        rawStateTransition.protocolVersion,
      );
    });
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawStateTransition.type;

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('type');
    });

    it('should be equal to 5', async () => {
      rawStateTransition.type = 666;

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

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

      expectJsonSchemaError(result);

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

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/identityId/0');
      expect(error.getKeyword()).to.equal('type');

      expect(byteArrayError.getKeyword()).to.equal('byteArray');
    });

    it('should be no less than 32 bytes', async () => {
      rawStateTransition.identityId = Buffer.alloc(31);

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/identityId');
      expect(error.getKeyword()).to.equal('minItems');
    });

    it('should be no longer than 32 bytes', async () => {
      rawStateTransition.identityId = Buffer.alloc(33);

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/identityId');
      expect(error.getKeyword()).to.equal('maxItems');
    });
  });

  describe('signature', () => {
    it('should be present', async () => {
      delete rawStateTransition.signature;

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('signature');
    });

    it('should be a byte array', async () => {
      rawStateTransition.signature = new Array(65).fill('string');

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.instancePath).to.equal('/signature/0');
      expect(error.getKeyword()).to.equal('type');

      expect(byteArrayError.getKeyword()).to.equal('byteArray');
    });

    it('should be not shorter than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(64);

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/signature');
      expect(error.getKeyword()).to.equal('minItems');
    });

    it('should be not longer than 96 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(97);

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/signature');
      expect(error.getKeyword()).to.equal('maxItems');
    });
  });

  describe('revision', () => {
    it('should be present', async () => {
      delete rawStateTransition.revision;

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('revision');
    });

    it('should be integer', async () => {
      rawStateTransition.revision = '1';

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/revision');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be greater or equal 0', async () => {
      rawStateTransition.revision = -1;

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

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
      rawStateTransition.addPublicKeys = [publicKeyToAdd];

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      expect(result.isValid()).to.be.true();
    });

    it('should not be empty', async () => {
      rawStateTransition.addPublicKeys = [];

      const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();
      expect(error.getKeyword()).to.equal('minItems');
      expect(error.getInstancePath()).to.equal('/addPublicKeys');
    });

    it('should not have more than 10 items', async () => {
      rawStateTransition.addPublicKeys = [];

      for (let i = 0; i <= 10; i++) {
        rawStateTransition.addPublicKeys.push(publicKeyToAdd);
      }

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('maxItems');
      expect(error.getInstancePath()).to.equal('/addPublicKeys');
    });

    it('should be unique', async () => {
      rawStateTransition.addPublicKeys = [publicKeyToAdd, publicKeyToAdd];

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('uniqueItems');
      expect(error.getInstancePath()).to.equal('/addPublicKeys');
    });

    it('should be valid', async () => {
      rawStateTransition.addPublicKeys = [publicKeyToAdd];

      const publicKeysError = new SomeConsensusError('test');
      const publicKeysResult = new ValidationResult([
        publicKeysError,
      ]);

      validatePublicKeysMock.returns(publicKeysResult);

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      expectValidationError(result);

      const [error] = result.getErrors();

      expect(error).to.equal(publicKeysError);

      expect(validatePublicKeysMock).to.be.calledOnceWithExactly(
        rawStateTransition.addPublicKeys,
      );
    });

    it('should have valid signatures', async () => {
      const publicKeysError = new SomeConsensusError('test');
      const publicKeysResult = new ValidationResult([
        publicKeysError,
      ]);

      validatePublicKeySignaturesMock.resolves(publicKeysResult);

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      expectValidationError(result);

      const [error] = result.getErrors();

      expect(error).to.equal(publicKeysError);
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

      expectJsonSchemaError(result);

      const [error] = result.getErrors();
      expect(error.getKeyword()).to.equal('dependentRequired');
      expect(error.params.missingProperty).to.equal('publicKeysDisabledAt');
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

      expectJsonSchemaError(result);

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

      expectJsonSchemaError(result);

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

      expectJsonSchemaError(result);

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

      expectJsonSchemaError(result);

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

      expectJsonSchemaError(result);

      const [error] = result.getErrors();
      expect(error.getKeyword()).to.equal('dependentRequired');
      expect(error.params.missingProperty).to.equal('disablePublicKeys');
    });

    it('should be integer', async () => {
      rawStateTransition.publicKeysDisabledAt = 1.1;
      rawStateTransition.disablePublicKeys = [0];

      const result = await validateIdentityUpdateTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

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

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/publicKeysDisabledAt');
      expect(error.getKeyword()).to.equal('minimum');
    });
  });

  it('should return valid result', async () => {
    const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

    expect(result.isValid()).to.be.true();

    expect(validatePublicKeysMock)
      .to.be.calledOnceWithExactly(
        rawStateTransition.addPublicKeys,
      );
  });

  it('should have either addPublicKeys or disablePublicKeys', async () => {
    delete rawStateTransition.disablePublicKeys;
    delete rawStateTransition.addPublicKeys;
    delete rawStateTransition.publicKeysDisabledAt;

    const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

    expectJsonSchemaError(result, 3);

    const [addPublicKeysError, disablePublicKeysError, error] = result.getErrors();

    expect(error.getKeyword()).to.equal('anyOf');

    expect(disablePublicKeysError.schemaPath).to.equal('#/anyOf/1/required');
    expect(disablePublicKeysError.getKeyword()).to.equal('required');

    expect(addPublicKeysError.schemaPath).to.equal('#/anyOf/0/required');
    expect(addPublicKeysError.getKeyword()).to.equal('required');
  });
});
