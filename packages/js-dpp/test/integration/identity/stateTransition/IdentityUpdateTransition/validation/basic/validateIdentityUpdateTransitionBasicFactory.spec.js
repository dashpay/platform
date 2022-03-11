const { default: getRE2Class } = require('@dashevo/re2-wasm');

const validateIdentityUpdateTransitionBasicFactory = require(
  '../../../../../../../lib/identity/stateTransition/IdentityUpdateTransition/validation/basic/validateIdentityUpdateTransitionBasicFactory',
);
const JsonSchemaValidator = require('../../../../../../../lib/validation/JsonSchemaValidator');
const createAjv = require('../../../../../../../lib/ajv/createAjv');
const InstantAssetLockProof = require('../../../../../../../lib/identity/stateTransition/assetLockProof/instant/InstantAssetLockProof');
const ValidationResult = require('../../../../../../../lib/validation/ValidationResult');
const IdentityPublicKey = require('../../../../../../../lib/identity/IdentityPublicKey');
const getIdentityUpdateTransitionFixture = require('../../../../../../../lib/test/fixtures/getIdentityUpdateTransitionFixture');
const { expectJsonSchemaError, expectValidationError} = require('../../../../../../../lib/test/expect/expectError');
const SomeConsensusError = require('../../../../../../../lib/test/mocks/SomeConsensusError');

describe('validateIdentityUpdateTransitionBasicFactory.spec', () => {
  let validateIdentityUpdateTransitionBasic;
  let validateProtocolVersionMock;
  let validatePublicKeysMock;
  let rawStateTransition;
  let stateTransition;

  beforeEach(async function beforeEach() {
    const RE2 = await getRE2Class();
    const ajv = createAjv(RE2);
    const jsonSchemaValidator = new JsonSchemaValidator(ajv);

    validateProtocolVersionMock = this.sinonSandbox.stub().returns(new ValidationResult());

    validatePublicKeysMock = this.sinonSandbox.stub()
      .returns(new ValidationResult());

    validateIdentityUpdateTransitionBasic = validateIdentityUpdateTransitionBasicFactory(
      jsonSchemaValidator,
      validateProtocolVersionMock,
      validatePublicKeysMock,
    );

    stateTransition = getIdentityUpdateTransitionFixture();

    const privateKey = '9b67f852093bc61cea0eeca38599dbfba0de28574d2ed9b99d10d33dc1bde7b2';

    await stateTransition.signByPrivateKey(privateKey, IdentityPublicKey.TYPES.ECDSA_SECP256K1);

    rawStateTransition = stateTransition.toObject();
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

    it('should be not longer than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(66);

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
    it('should not be empty', async () => {

    });

    it('should not have more than 10 items', async () => {

    });

    it('should be unique', async () => {

    });

    it('should be valid', async () => {

    });

    // TODO master key ????
  });

  describe('disablePublicKeys', async () => {
    it('should be valid', async () => {

    });

    it('should contain integers', async () => {

    });

    it('should not have more than 10 items', async () => {

    });

    it('should be unique', async () => {

    });

    it('should be used only with publicKeysDisabledAt', async () => {

    });
  });

  describe('publicKeysDisabledAt', async () => {
    it('should be integer', async () => {

    });

    it('should be valid', async () => {

    });

    it('should be used only with disablePublicKeys', async () => {

    });
  });

  it('should return valid result', async () => {
    const result = await validateIdentityUpdateTransitionBasic(rawStateTransition);

    expect(result.isValid()).to.be.true();

    expect(proofValidationFunctionsByTypeMock[InstantAssetLockProof.type])
      .to.be.calledOnceWithExactly(
      rawStateTransition.assetLockProof,
    );
  });
});
