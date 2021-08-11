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

const ConsensusError = require('../../../../../../../lib/errors/ConsensusError');
const IdentityNotFoundError = require('../../../../../../../lib/errors/IdentityNotFoundError');
const Identifier = require('../../../../../../../lib/identifier/Identifier');
const ChainAssetLockProof = require('../../../../../../../lib/identity/stateTransition/assetLockProof/chain/ChainAssetLockProof');
const InstantAssetLockProof = require('../../../../../../../lib/identity/stateTransition/assetLockProof/instant/InstantAssetLockProof');

describe('validateIdentityTopUpTransitionBasicFactory', () => {
  let rawStateTransition;
  let stateTransition;
  let assetLockPublicKeyHash;
  let validateIdentityTopUpTransitionBasic;
  let validateIdentityExistenceMock;
  let proofValidationFunctionsByTypeMock;

  beforeEach(async function beforeEach() {
    validateIdentityExistenceMock = this.sinonSandbox.stub().resolves(new ValidationResult());

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

    validateIdentityTopUpTransitionBasic = validateIdentityTopUpTransitionBasicFactory(
      jsonSchemaValidator,
      validateIdentityExistenceMock,
      proofValidationFunctionsByTypeMock,
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

      expect(error.instancePath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('protocolVersion');
    });

    it('should be an integer', async () => {
      rawStateTransition.protocolVersion = '1';

      const result = await validateIdentityTopUpTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/protocolVersion');
      expect(error.keyword).to.equal('type');
    });

    it('should not be less than 0', async () => {
      rawStateTransition.protocolVersion = -1;

      const result = await validateIdentityTopUpTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('minimum');
      expect(error.instancePath).to.equal('/protocolVersion');
    });

    it('should not be greater than current version (0)', async () => {
      rawStateTransition.protocolVersion = 1;

      const result = await validateIdentityTopUpTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('maximum');
      expect(error.instancePath).to.equal('/protocolVersion');
    });
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawStateTransition.type;

      const result = await validateIdentityTopUpTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('type');
    });

    it('should be equal to 3', async () => {
      rawStateTransition.type = 666;

      const result = await validateIdentityTopUpTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/type');
      expect(error.keyword).to.equal('const');
      expect(error.params.allowedValue).to.equal(3);
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

      expect(error.instancePath).to.equal('');
      expect(error.params.missingProperty).to.equal('assetLockProof');
      expect(error.keyword).to.equal('required');
    });

    it('should be an object', async () => {
      rawStateTransition.assetLockProof = 1;

      const result = await validateIdentityTopUpTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/assetLockProof');
      expect(error.keyword).to.equal('type');
    });

    it('should be valid', async () => {
      const assetLockError = new ConsensusError('test');
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

      expect(error.instancePath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('identityId');
    });

    it('should be a byte array', async () => {
      rawStateTransition.identityId = new Array(32).fill('string');

      const result = await validateIdentityTopUpTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.instancePath).to.equal('/identityId/0');
      expect(error.keyword).to.equal('type');

      expect(byteArrayError.keyword).to.equal('byteArray');
    });

    it('should be no less than 32 bytes', async () => {
      rawStateTransition.identityId = Buffer.alloc(31);

      const result = await validateIdentityTopUpTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/identityId');
      expect(error.keyword).to.equal('minItems');
    });

    it('should be no longer than 32 bytes', async () => {
      rawStateTransition.identityId = Buffer.alloc(33);

      const result = await validateIdentityTopUpTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/identityId');
      expect(error.keyword).to.equal('maxItems');
    });

    it('should exist', async () => {
      const identityNotFoundResult = new ValidationResult([
        new IdentityNotFoundError(stateTransition.getIdentityId()),
      ]);

      validateIdentityExistenceMock.resolves(identityNotFoundResult);

      const result = await validateIdentityTopUpTransitionBasic(rawStateTransition);

      expectValidationError(result, IdentityNotFoundError);

      const [error] = result.getErrors();

      expect(error.getIdentityId()).to.be.equal(stateTransition.getIdentityId());

      expect(validateIdentityExistenceMock).to.be.calledOnce();

      expect(validateIdentityExistenceMock.getCall(0).args).to.have.lengthOf(1);

      const [identityId] = validateIdentityExistenceMock.getCall(0).args;

      expect(identityId).to.be.instanceOf(Identifier);
      expect(identityId).to.deep.equal(stateTransition.getIdentityId());
    });
  });

  describe('signature', () => {
    it('should be present', async () => {
      delete rawStateTransition.signature;

      const result = await validateIdentityTopUpTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('signature');
    });

    it('should be a byte array', async () => {
      rawStateTransition.signature = new Array(65).fill('string');

      const result = await validateIdentityTopUpTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.instancePath).to.equal('/signature/0');
      expect(error.keyword).to.equal('type');

      expect(byteArrayError.keyword).to.equal('byteArray');
    });

    it('should be not shorter than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(64);

      const result = await validateIdentityTopUpTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/signature');
      expect(error.keyword).to.equal('minItems');
    });

    it('should be not longer than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(66);

      const result = await validateIdentityTopUpTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/signature');
      expect(error.keyword).to.equal('maxItems');
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
