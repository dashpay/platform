const { default: getRE2Class } = require('@dashevo/re2-wasm');

const validateIdentityUpdateTransitionBasicFactory = require(
  '../../../../../../../lib/identity/stateTransition/IdentityUpdateTransition/validation/basic/validateIdentityUpdateTransitionBasicFactory',
);
const JsonSchemaValidator = require('../../../../../../../lib/validation/JsonSchemaValidator');
const createAjv = require('../../../../../../../lib/ajv/createAjv');
const InstantAssetLockProof = require('../../../../../../../lib/identity/stateTransition/assetLockProof/instant/InstantAssetLockProof');
const ChainAssetLockProof = require('../../../../../../../lib/identity/stateTransition/assetLockProof/chain/ChainAssetLockProof');
const ValidationResult = require('../../../../../../../lib/validation/ValidationResult');

describe('validateIdentityUpdateTransitionBasicFactory.spec', () => {
  let validateIdentityUpdateTransitionBasic;
  let proofValidationFunctionsByTypeMock;
  let assetLockPublicKeyHash;
  let validateProtocolVersionMock;
  let validatePublicKeysMock;

  beforeEach(async function beforeEach() {
    const RE2 = await getRE2Class();
    const ajv = createAjv(RE2);
    const jsonSchemaValidator = new JsonSchemaValidator(ajv);

    assetLockPublicKeyHash = Buffer.alloc(20, 1);

    const assetLockValidationResult = new ValidationResult();
    assetLockValidationResult.setData(assetLockPublicKeyHash);

    proofValidationFunctionsByTypeMock = {
      [InstantAssetLockProof.type]: this.sinonSandbox.stub().resolves(assetLockValidationResult),
      [ChainAssetLockProof.type]: this.sinonSandbox.stub().resolves(assetLockValidationResult),
    };

    validateProtocolVersionMock = this.sinonSandbox.stub().returns(new ValidationResult());

    validatePublicKeysMock = this.sinonSandbox.stub()
      .returns(new ValidationResult());

    validateIdentityUpdateTransitionBasic = validateIdentityUpdateTransitionBasicFactory(
      jsonSchemaValidator,
      proofValidationFunctionsByTypeMock,
      validateProtocolVersionMock,
      validatePublicKeysMock,
    );
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {

    });

    it('should be integer', async () => {

    });

    it('should be valid', async () => {

    });
  });

  describe('type', () => {
    it('should be present', async () => {

    });

    it('should be equal to 5', async () => {

    });
  });

  describe('assetLockProof', () => {
    it('should be present', async () => {

    });

    it('should be an object', async () => {

    });

    it('should be valid', async () => {

    });
  });

  describe('identityId', () => {
    it('should be present', async () => {

    });

    it('should be a byte array', async () => {

    });

    it('should be no less than 32 bytes', async () => {

    });

    it('should be no longer than 32 bytes', async () => {

    });
  });

  describe('signature', () => {
    it('should be present', async () => {

    });

    it('should be a byte array', async () => {

    });

    it('should be not shorter than 65 bytes', async () => {

    });

    it('should be not longer than 65 bytes', async () => {

    });
  });

  describe('revision', () => {
    it('should be present', async () => {

    });

    it('should be integer', async () => {

    });

    it('should be valid', async () => {

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

  });
});
