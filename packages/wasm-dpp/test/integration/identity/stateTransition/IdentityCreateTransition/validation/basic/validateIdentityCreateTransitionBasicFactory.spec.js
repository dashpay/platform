const { getRE2Class } = require('@dashevo/wasm-re2');

const createAjv = require('../../../../../../../lib/ajv/createAjv');

const JsonSchemaValidator = require('../../../../../../../lib/validation/JsonSchemaValidator');

const getIdentityCreateTransitionFixture = require('../../../../../../../lib/test/fixtures/getIdentityCreateTransitionFixture');

const validateIdentityCreateTransitionBasicFactory = require(
  '../../../../../../../lib/identity/stateTransition/IdentityCreateTransition/validation/basic/validateIdentityCreateTransitionBasicFactory',
);

const {
  expectJsonSchemaError,
  expectValidationError,
} = require('../../../../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../../../../lib/validation/ValidationResult');
const InstantAssetLockProof = require('../../../../../../../lib/identity/stateTransition/assetLockProof/instant/InstantAssetLockProof');
const ChainAssetLockProof = require('../../../../../../../lib/identity/stateTransition/assetLockProof/chain/ChainAssetLockProof');
const SomeConsensusError = require('../../../../../../../lib/test/mocks/SomeConsensusError');
const IdentityPublicKey = require('../../../../../../../lib/identity/IdentityPublicKey');
const StateTransitionExecutionContext = require('../../../../../../../lib/stateTransition/StateTransitionExecutionContext');

describe('validateIdentityCreateTransitionBasicFactory', () => {
  let validateIdentityCreateTransitionBasic;
  let rawStateTransition;
  let stateTransition;
  let validatePublicKeysMock;
  let validatePublicKeysInIdentityCreateTransition;
  let assetLockPublicKeyHash;
  let proofValidationFunctionsByTypeMock;
  let validateProtocolVersionMock;
  let validatePublicKeySignaturesMock;

  beforeEach(async function beforeEach() {
    validatePublicKeysMock = this.sinonSandbox.stub()
      .returns(new ValidationResult());

    validatePublicKeysInIdentityCreateTransition = this.sinonSandbox.stub()
      .returns(new ValidationResult());

    assetLockPublicKeyHash = Buffer.alloc(20, 1);

    const assetLockValidationResult = new ValidationResult();

    assetLockValidationResult.setData(assetLockPublicKeyHash);

    const RE2 = await getRE2Class();
    const ajv = createAjv(RE2);

    const jsonSchemaValidator = new JsonSchemaValidator(ajv);

    const proofValidationResult = new ValidationResult();
    proofValidationResult.setData(assetLockPublicKeyHash);

    proofValidationFunctionsByTypeMock = {
      [InstantAssetLockProof.type]: this.sinonSandbox.stub().resolves(proofValidationResult),
      [ChainAssetLockProof.type]: this.sinonSandbox.stub().resolves(proofValidationResult),
    };

    validateProtocolVersionMock = this.sinonSandbox.stub().returns(new ValidationResult());

    validatePublicKeySignaturesMock = this.sinonSandbox.stub()
      .returns(new ValidationResult());

    validateIdentityCreateTransitionBasic = validateIdentityCreateTransitionBasicFactory(
      jsonSchemaValidator,
      validatePublicKeysMock,
      validatePublicKeysInIdentityCreateTransition,
      proofValidationFunctionsByTypeMock,
      validateProtocolVersionMock,
      validatePublicKeySignaturesMock,
    );

    stateTransition = getIdentityCreateTransitionFixture();

    const privateKey = '9b67f852093bc61cea0eeca38599dbfba0de28574d2ed9b99d10d33dc1bde7b2';

    await stateTransition.signByPrivateKey(privateKey, IdentityPublicKey.TYPES.ECDSA_SECP256K1);

    rawStateTransition = stateTransition.toObject();
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {
      delete rawStateTransition.protocolVersion;

      const result = await validateIdentityCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('protocolVersion');
    });

    it('should be an integer', async () => {
      rawStateTransition.protocolVersion = '1';

      const result = await validateIdentityCreateTransitionBasic(rawStateTransition);

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

      const result = await validateIdentityCreateTransitionBasic(rawStateTransition);

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

      const result = await validateIdentityCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('type');
    });

    it('should be equal to 2', async () => {
      rawStateTransition.type = 666;

      const result = await validateIdentityCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

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

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getParams().missingProperty).to.equal('assetLockProof');
      expect(error.getKeyword()).to.equal('required');
    });

    it('should be an object', async () => {
      rawStateTransition.assetLockProof = 1;

      const result = await validateIdentityCreateTransitionBasic(rawStateTransition);

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

      const executionContext = new StateTransitionExecutionContext();

      proofValidationFunctionsByTypeMock[InstantAssetLockProof.type].resolves(assetLockResult);

      const result = await validateIdentityCreateTransitionBasic(
        rawStateTransition,
        executionContext,
      );

      expectValidationError(result);

      const [error] = result.getErrors();

      expect(error).to.equal(assetLockError);

      expect(proofValidationFunctionsByTypeMock[InstantAssetLockProof.type])
        .to.be.calledOnceWithExactly(
          rawStateTransition.assetLockProof,
          executionContext,
        );
    });
  });

  describe('publicKeys', () => {
    it('should be present', async () => {
      rawStateTransition.publicKeys = undefined;

      const result = await validateIdentityCreateTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

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

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('minItems');
      expect(error.getInstancePath()).to.equal('/publicKeys');
    });

    it('should not have more than 10 items', async () => {
      const [key] = rawStateTransition.publicKeys;

      for (let i = 0; i < 10; i++) {
        rawStateTransition.publicKeys.push(key);
      }

      const result = await validateIdentityCreateTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('maxItems');
      expect(error.getInstancePath()).to.equal('/publicKeys');
    });

    it('should be unique', async () => {
      rawStateTransition.publicKeys.push(rawStateTransition.publicKeys[0]);

      const result = await validateIdentityCreateTransitionBasic(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('uniqueItems');
      expect(error.getInstancePath()).to.equal('/publicKeys');
    });

    it('should be valid', async () => {
      const publicKeysError = new SomeConsensusError('test');
      const publicKeysResult = new ValidationResult([
        publicKeysError,
      ]);

      validatePublicKeysMock.returns(publicKeysResult);

      const result = await validateIdentityCreateTransitionBasic(
        rawStateTransition,
      );

      expectValidationError(result);

      const [error] = result.getErrors();

      expect(error).to.equal(publicKeysError);

      expect(validatePublicKeysMock)
        .to.be.calledOnceWithExactly(rawStateTransition.publicKeys);
    });

    it('should have at least 1 master key', async () => {
      const publicKeysError = new SomeConsensusError('test');
      const publicKeysResult = new ValidationResult([
        publicKeysError,
      ]);

      validatePublicKeysInIdentityCreateTransition.returns(publicKeysResult);

      const result = await validateIdentityCreateTransitionBasic(
        rawStateTransition,
      );

      expectValidationError(result);

      const [error] = result.getErrors();

      expect(error).to.equal(publicKeysError);

      expect(validatePublicKeysInIdentityCreateTransition)
        .to.be.calledOnceWithExactly(rawStateTransition.publicKeys);
    });

    it('should have valid signatures', async () => {
      const publicKeysError = new SomeConsensusError('test');
      const publicKeysResult = new ValidationResult([
        publicKeysError,
      ]);

      validatePublicKeySignaturesMock.resolves(publicKeysResult);

      const result = await validateIdentityCreateTransitionBasic(
        rawStateTransition,
      );

      expectValidationError(result);

      const [error] = result.getErrors();

      expect(error).to.equal(publicKeysError);
    });
  });

  describe('signature', () => {
    it('should be present', async () => {
      delete rawStateTransition.signature;

      const result = await validateIdentityCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('signature');
    });

    it('should be a byte array', async () => {
      rawStateTransition.signature = new Array(65).fill('string');

      const result = await validateIdentityCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.instancePath).to.equal('/signature/0');
      expect(error.getKeyword()).to.equal('type');

      expect(byteArrayError.getKeyword()).to.equal('byteArray');
    });

    it('should be not shorter than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(64);

      const result = await validateIdentityCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/signature');
      expect(error.getKeyword()).to.equal('minItems');
    });

    it('should be not longer than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(66);

      const result = await validateIdentityCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/signature');
      expect(error.getKeyword()).to.equal('maxItems');
    });
  });

  it('should return valid result', async () => {
    const result = await validateIdentityCreateTransitionBasic(rawStateTransition);

    expect(result.isValid()).to.be.true();

    expect(validatePublicKeysMock).to.be.calledOnceWithExactly(
      rawStateTransition.publicKeys,
    );
  });
});
