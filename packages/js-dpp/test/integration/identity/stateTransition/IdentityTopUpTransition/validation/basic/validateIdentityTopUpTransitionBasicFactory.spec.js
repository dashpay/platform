const { default: getRE2Class } = require('@dashevo/re2-wasm');

const createAjv = require('../../../../../../../lib/ajv/createAjv');

const JsonSchemaValidator = require('../../../../../../../lib/validation/JsonSchemaValidator');

const getIdentityTopUpTransitionFixture = require('../../../../../../../lib/test/fixtures/getIdentityTopUpTransitionFixture');

const validateIdentityTopUpTransitionBasicFactory = require(
  '../../../../../../../lib/identity/stateTransition/IdentityTopUpTransition/validation/basic/validateIdentityTopUpTransitionBasicFactory',
);

const {
  expectJsonSchemaError,
  expectValidationError,
} = require('../../../../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../../../../lib/validation/ValidationResult');

const ChainAssetLockProof = require('../../../../../../../lib/identity/stateTransition/assetLockProof/chain/ChainAssetLockProof');
const InstantAssetLockProof = require('../../../../../../../lib/identity/stateTransition/assetLockProof/instant/InstantAssetLockProof');
const SomeConsensusError = require('../../../../../../../lib/test/mocks/SomeConsensusError');

describe('validateIdentityTopUpTransitionBasicFactory', () => {
  let rawStateTransition;
  let stateTransition;
  let assetLockPublicKeyHash;
  let validateIdentityTopUpTransitionBasic;
  let proofValidationFunctionsByTypeMock;
  let validateProtocolVersionMock;

  beforeEach(async function beforeEach() {
    assetLockPublicKeyHash = Buffer.alloc(20, 1);

    const assetLockValidationResult = new ValidationResult();
    assetLockValidationResult.setData(assetLockPublicKeyHash);

    proofValidationFunctionsByTypeMock = {
      [InstantAssetLockProof.type]: this.sinonSandbox.stub().resolves(assetLockValidationResult),
      [ChainAssetLockProof.type]: this.sinonSandbox.stub().resolves(assetLockValidationResult),
    };

    const RE2 = await getRE2Class();
    const ajv = createAjv(RE2);

    const jsonSchemaValidator = new JsonSchemaValidator(ajv);

    validateProtocolVersionMock = this.sinonSandbox.stub().returns(new ValidationResult());

    validateIdentityTopUpTransitionBasic = validateIdentityTopUpTransitionBasicFactory(
      jsonSchemaValidator,
      proofValidationFunctionsByTypeMock,
      validateProtocolVersionMock,
    );

    stateTransition = getIdentityTopUpTransitionFixture();

    const privateKey = '9b67f852093bc61cea0eeca38599dbfba0de28574d2ed9b99d10d33dc1bde7b2';

    stateTransition.signByPrivateKey(privateKey);

    rawStateTransition = stateTransition.toObject();
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {
      delete rawStateTransition.protocolVersion;

      const result = await validateIdentityTopUpTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('protocolVersion');
    });

    it('should be an integer', async () => {
      rawStateTransition.protocolVersion = '1';

      const result = await validateIdentityTopUpTransitionBasic(rawStateTransition);

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

      const result = await validateIdentityTopUpTransitionBasic(rawStateTransition);

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

      const result = await validateIdentityTopUpTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('type');
    });

    it('should be equal to 3', async () => {
      rawStateTransition.type = 666;

      const result = await validateIdentityTopUpTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

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
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getParams().missingProperty).to.equal('assetLockProof');
      expect(error.getKeyword()).to.equal('required');
    });

    it('should be an object', async () => {
      rawStateTransition.assetLockProof = 1;

      const result = await validateIdentityTopUpTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/assetLockProof');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be valid', async () => {
      const assetLockError = new SomeConsensusError('test');
      const assetLockResult = new ValidationResult([
        assetLockError,
      ]);

      proofValidationFunctionsByTypeMock[InstantAssetLockProof.type].resolves(assetLockResult);

      const result = await validateIdentityTopUpTransitionBasic(
        rawStateTransition,
      );

      expectValidationError(result);

      const [error] = result.getErrors();

      expect(error).to.equal(assetLockError);

      expect(proofValidationFunctionsByTypeMock[InstantAssetLockProof.type])
        .to.be.calledOnceWithExactly(
          rawStateTransition.assetLockProof,
        );
    });
  });

  describe('identityId', () => {
    it('should be present', async () => {
      delete rawStateTransition.identityId;

      const result = await validateIdentityTopUpTransitionBasic(
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

      const result = await validateIdentityTopUpTransitionBasic(
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

      const result = await validateIdentityTopUpTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/identityId');
      expect(error.getKeyword()).to.equal('minItems');
    });

    it('should be no longer than 32 bytes', async () => {
      rawStateTransition.identityId = Buffer.alloc(33);

      const result = await validateIdentityTopUpTransitionBasic(
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

      const result = await validateIdentityTopUpTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('signature');
    });

    it('should be a byte array', async () => {
      rawStateTransition.signature = new Array(65).fill('string');

      const result = await validateIdentityTopUpTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.instancePath).to.equal('/signature/0');
      expect(error.getKeyword()).to.equal('type');

      expect(byteArrayError.getKeyword()).to.equal('byteArray');
    });

    it('should be not shorter than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(64);

      const result = await validateIdentityTopUpTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/signature');
      expect(error.getKeyword()).to.equal('minItems');
    });

    it('should be not longer than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(66);

      const result = await validateIdentityTopUpTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/signature');
      expect(error.getKeyword()).to.equal('maxItems');
    });
  });

  it('should return valid result', async () => {
    const result = await validateIdentityTopUpTransitionBasic(rawStateTransition);

    expect(result.isValid()).to.be.true();

    expect(proofValidationFunctionsByTypeMock[InstantAssetLockProof.type])
      .to.be.calledOnceWithExactly(
        rawStateTransition.assetLockProof,
      );
  });
});
