const createAjv = require('../../../../../lib/ajv/createAjv');

const JsonSchemaValidator = require('../../../../../lib/validation/JsonSchemaValidator');

const getIdentityTopUpTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityTopUpTransitionFixture');

const validateIdentityTopUpTransitionStructureFactory = require(
  '../../../../../lib/identity/stateTransitions/identityTopUpTransition/validateIdentityTopUpTransitionStructureFactory',
);

const {
  expectJsonSchemaError,
  expectValidationError,
} = require('../../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../../lib/validation/ValidationResult');

const ConsensusError = require('../../../../../lib/errors/ConsensusError');
const IdentityNotFoundError = require('../../../../../lib/errors/IdentityNotFoundError');
const Identifier = require('../../../../../lib/identifier/Identifier');
const ChainAssetLockProof = require('../../../../../lib/identity/stateTransitions/assetLockProof/chain/ChainAssetLockProof');
const InstantAssetLockProof = require('../../../../../lib/identity/stateTransitions/assetLockProof/instant/InstantAssetLockProof');

describe('validateIdentityTopUpTransitionStructureFactory', () => {
  let rawStateTransition;
  let stateTransition;
  let assetLockPublicKeyHash;
  let validateIdentityTopUpTransitionStructure;
  let validateSignatureAgainstAssetLockPublicKeyMock;
  let validateIdentityExistenceMock;
  let proofValidationFunctionsByTypeMock;

  beforeEach(function beforeEach() {
    validateIdentityExistenceMock = this.sinonSandbox.stub().resolves(new ValidationResult());

    assetLockPublicKeyHash = Buffer.alloc(20, 1);

    const assetLockValidationResult = new ValidationResult();
    assetLockValidationResult.setData(assetLockPublicKeyHash);

    proofValidationFunctionsByTypeMock = {
      [InstantAssetLockProof.type]: this.sinonSandbox.stub().resolves(assetLockValidationResult),
      [ChainAssetLockProof.type]: this.sinonSandbox.stub().resolves(assetLockValidationResult),
    };

    validateSignatureAgainstAssetLockPublicKeyMock = this.sinonSandbox.stub()
      .resolves(new ValidationResult());

    const ajv = createAjv();
    const jsonSchemaValidator = new JsonSchemaValidator(ajv);

    validateIdentityTopUpTransitionStructure = validateIdentityTopUpTransitionStructureFactory(
      jsonSchemaValidator,
      validateIdentityExistenceMock,
      validateSignatureAgainstAssetLockPublicKeyMock,
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

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('protocolVersion');
    });

    it('should be an integer', async () => {
      rawStateTransition.protocolVersion = '1';

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/protocolVersion');
      expect(error.keyword).to.equal('type');
    });

    it('should not be less than 0', async () => {
      rawStateTransition.protocolVersion = -1;

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('minimum');
      expect(error.dataPath).to.equal('/protocolVersion');
    });

    it('should not be greater than current version (0)', async () => {
      rawStateTransition.protocolVersion = 1;

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('maximum');
      expect(error.dataPath).to.equal('/protocolVersion');
    });
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawStateTransition.type;

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('type');
    });

    it('should be equal to 3', async () => {
      rawStateTransition.type = 666;

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/type');
      expect(error.keyword).to.equal('const');
      expect(error.params.allowedValue).to.equal(3);
    });
  });

  describe('assetLockProof', () => {
    it('should be present', async () => {
      delete rawStateTransition.assetLockProof;

      const result = await validateIdentityTopUpTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.params.missingProperty).to.equal('assetLockProof');
      expect(error.keyword).to.equal('required');
    });

    it('should be an object', async () => {
      rawStateTransition.assetLockProof = 1;

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/assetLockProof');
      expect(error.keyword).to.equal('type');
    });

    it('should be valid', async () => {
      const assetLockError = new ConsensusError('test');
      const assetLockResult = new ValidationResult([
        assetLockError,
      ]);

      proofValidationFunctionsByTypeMock[InstantAssetLockProof.type].resolves(assetLockResult);

      const result = await validateIdentityTopUpTransitionStructure(
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

      const result = await validateIdentityTopUpTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('identityId');
    });

    it('should be a byte array', async () => {
      rawStateTransition.identityId = new Array(32).fill('string');

      const result = await validateIdentityTopUpTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.dataPath).to.equal('/identityId/0');
      expect(error.keyword).to.equal('type');

      expect(byteArrayError.keyword).to.equal('byteArray');
    });

    it('should be no less than 32 bytes', async () => {
      rawStateTransition.identityId = Buffer.alloc(31);

      const result = await validateIdentityTopUpTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/identityId');
      expect(error.keyword).to.equal('minItems');
    });

    it('should be no longer than 32 bytes', async () => {
      rawStateTransition.identityId = Buffer.alloc(33);

      const result = await validateIdentityTopUpTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/identityId');
      expect(error.keyword).to.equal('maxItems');
    });

    it('should exist', async () => {
      const identityNotFoundResult = new ValidationResult([
        new IdentityNotFoundError(stateTransition.getIdentityId()),
      ]);

      validateIdentityExistenceMock.resolves(identityNotFoundResult);

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

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

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('signature');
    });

    it('should be a byte array', async () => {
      rawStateTransition.signature = new Array(65).fill('string');

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.dataPath).to.equal('/signature/0');
      expect(error.keyword).to.equal('type');

      expect(byteArrayError.keyword).to.equal('byteArray');
    });

    it('should be not shorter than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(64);

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/signature');
      expect(error.keyword).to.equal('minItems');
    });

    it('should be not longer than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(66);

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/signature');
      expect(error.keyword).to.equal('maxItems');
    });

    it('should be valid', async () => {
      const signatureError = new ConsensusError('test');
      const signatureResult = new ValidationResult([
        signatureError,
      ]);

      validateSignatureAgainstAssetLockPublicKeyMock.returns(signatureResult);

      const result = await validateIdentityTopUpTransitionStructure(
        rawStateTransition,
      );

      expectValidationError(result);

      const [error] = result.getErrors();

      expect(error).to.equal(signatureError);

      expect(validateSignatureAgainstAssetLockPublicKeyMock).to.be.calledOnceWithExactly(
        rawStateTransition,
        assetLockPublicKeyHash,
      );
    });
  });

  it('should return valid result', async () => {
    const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

    expect(result.isValid()).to.be.true();

    expect(proofValidationFunctionsByTypeMock[InstantAssetLockProof.type])
      .to.be.calledOnceWithExactly(
        rawStateTransition.assetLockProof,
      );
  });
});
