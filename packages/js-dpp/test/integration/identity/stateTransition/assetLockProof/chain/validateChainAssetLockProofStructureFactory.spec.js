const createAjv = require('../../../../../../lib/ajv/createAjv');

const getChainAssetLockFixture = require('../../../../../../lib/test/fixtures/getChainAssetLockProofFixture');
const JsonSchemaValidator = require('../../../../../../lib/validation/JsonSchemaValidator');
const createStateRepositoryMock = require('../../../../../../lib/test/mocks/createStateRepositoryMock');

const { expectValidationError, expectJsonSchemaError } = require(
  '../../../../../../lib/test/expect/expectError',
);

const validateChainAssetLockProofStructureFactory = require('../../../../../../lib/identity/stateTransitions/assetLockProof/chain/validateChainAssetLockProofStructureFactory');
const ValidationResult = require('../../../../../../lib/validation/ValidationResult');
const IdentityAssetLockTransactionIsNotFoundError = require('../../../../../../lib/errors/IdentityAssetLockTransactionIsNotFoundError');
const ConsensusError = require('../../../../../../lib/errors/ConsensusError');
const InvalidAssetLockProofCoreChainHeightError = require('../../../../../../lib/errors/InvalidAssetLockProofCoreChainHeightError');
const InvalidAssetLockProofTransactionHeightError = require('../../../../../../lib/errors/InvalidAssetLockProofTransactionHeightError');

describe('validateChainAssetLockProofStructureFactory', () => {
  let rawProof;
  let stateRepositoryMock;
  let validateChainAssetLockProofStructure;
  let jsonSchemaValidator;
  let validateAssetLockTransactionMock;
  let validateAssetLockTransactionResult;
  let publicKeyHash;
  let rawTransaction;

  beforeEach(async function beforeEach() {
    rawTransaction = '030000000137feb5676d0851337ea3c9a992496aab7a0b3eee60aeeb9774000b7f4bababa5000000006b483045022100d91557de37645c641b948c6cd03b4ae3791a63a650db3e2fee1dcf5185d1b10402200e8bd410bf516ca61715867666d31e44495428ce5c1090bf2294a829ebcfa4ef0121025c3cc7fbfc52f710c941497fd01876c189171ea227458f501afcb38a297d65b4ffffffff021027000000000000166a14152073ca2300a86b510fa2f123d3ea7da3af68dcf77cb0090a0000001976a914152073ca2300a86b510fa2f123d3ea7da3af68dc88ac00000000';

    const assetLock = getChainAssetLockFixture();

    rawProof = assetLock.toObject();

    jsonSchemaValidator = new JsonSchemaValidator(await createAjv());

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    stateRepositoryMock.fetchLatestPlatformBlockHeader.resolves({
      coreChainLockedHeight: 42,
    });

    stateRepositoryMock.fetchTransaction.resolves({
      data: Buffer.from(rawTransaction, 'hex'),
      height: 42,
    });

    publicKeyHash = Buffer.from('152073ca2300a86b510fa2f123d3ea7da3af68dc', 'hex');

    validateAssetLockTransactionResult = new ValidationResult();
    validateAssetLockTransactionResult.setData({
      publicKeyHash,
    });
    validateAssetLockTransactionMock = this.sinonSandbox.stub().resolves(
      validateAssetLockTransactionResult,
    );

    validateChainAssetLockProofStructure = validateChainAssetLockProofStructureFactory(
      jsonSchemaValidator,
      stateRepositoryMock,
      validateAssetLockTransactionMock,
    );
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawProof.type;

      const result = await validateChainAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('type');

      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformBlockHeader).to.not.be.called();
    });

    it('should be equal to 1', async () => {
      rawProof.type = -1;

      const result = await validateChainAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/type');
      expect(error.keyword).to.equal('const');

      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformBlockHeader).to.not.be.called();
    });
  });

  describe('coreChainLockedHeight', () => {
    it('should be preset', async () => {
      delete rawProof.coreChainLockedHeight;

      const result = await validateChainAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('coreChainLockedHeight');

      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformBlockHeader).to.not.be.called();
    });

    it('should be an integer', async () => {
      rawProof.coreChainLockedHeight = 1.5;

      const result = await validateChainAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/coreChainLockedHeight');
      expect(error.keyword).to.equal('type');

      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformBlockHeader).to.not.be.called();
    });

    it('should be a number', async () => {
      rawProof.coreChainLockedHeight = '42';

      const result = await validateChainAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/coreChainLockedHeight');
      expect(error.keyword).to.equal('type');

      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformBlockHeader).to.not.be.called();
    });

    it('should be greater than 0', async () => {
      rawProof.coreChainLockedHeight = 0;

      const result = await validateChainAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/coreChainLockedHeight');
      expect(error.keyword).to.equal('minimum');

      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformBlockHeader).to.not.be.called();
    });

    it('should be less than 4294967296', async () => {
      rawProof.coreChainLockedHeight = 4294967296;

      const result = await validateChainAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/coreChainLockedHeight');
      expect(error.keyword).to.equal('maximum');

      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformBlockHeader).to.not.be.called();
    });

    it('should be less or equal to consensus core height', async () => {
      stateRepositoryMock.fetchLatestPlatformBlockHeader.resolves({
        coreChainLockedHeight: 41,
      });

      const result = await validateChainAssetLockProofStructure(rawProof);

      expectValidationError(result, InvalidAssetLockProofCoreChainHeightError);
      const [error] = result.getErrors();

      expect(error.getProofCoreChainLockedHeight()).to.equal(42);
      expect(error.getCurrentCoreChainLockedHeight()).to.equal(41);
    });
  });

  describe('outPoint', () => {
    it('should be present', async () => {
      delete rawProof.outPoint;

      const result = await validateChainAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('outPoint');

      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformBlockHeader).to.not.be.called();
    });

    it('should be a byte array', async () => {
      rawProof.outPoint = new Array(36).fill('string');

      const result = await validateChainAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.dataPath).to.equal('/outPoint/0');
      expect(error.keyword).to.equal('type');

      expect(byteArrayError.keyword).to.equal('byteArray');

      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformBlockHeader).to.not.be.called();
    });

    it('should not be shorter than 36 bytes', async () => {
      rawProof.outPoint = Buffer.alloc(35);

      const result = await validateChainAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/outPoint');
      expect(error.keyword).to.equal('minItems');

      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformBlockHeader).to.not.be.called();
    });

    it('should not be longer than 36 bytes', async () => {
      rawProof.outPoint = Buffer.alloc(37);

      const result = await validateChainAssetLockProofStructure(rawProof);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('/outPoint');
      expect(error.keyword).to.equal('maxItems');

      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
      expect(validateAssetLockTransactionMock).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformBlockHeader).to.not.be.called();
    });

    it('should point to existing transaction', async () => {
      stateRepositoryMock.fetchTransaction.resolves(null);

      const result = await validateChainAssetLockProofStructure(rawProof);

      expectValidationError(result, IdentityAssetLockTransactionIsNotFoundError);
      const [error] = result.getErrors();

      expect(error.getOutPoint()).to.deep.equal(rawProof.outPoint);

      expect(stateRepositoryMock.fetchTransaction).to.be.calledOnceWithExactly(
        '6e200d059fb567ba19e92f5c2dcd3dde522fd4e0a50af223752db16158dabb1d',
      );
      expect(validateAssetLockTransactionMock).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformBlockHeader).to.be.calledOnce();
    });

    it('should point to valid transaction', async () => {
      const consensusError = new ConsensusError('test');
      validateAssetLockTransactionResult.addError(consensusError);

      const result = await validateChainAssetLockProofStructure(rawProof);

      expectValidationError(result, ConsensusError);
      const [error] = result.getErrors();

      expect(error).to.deep.equal(consensusError);
    });

    it('should point to transaction from block lower than core chain locked height', async () => {
      rawProof.coreChainLockedHeight = 41;
      stateRepositoryMock.fetchLatestPlatformBlockHeader.resolves({
        coreChainLockedHeight: 41,
      });

      const result = await validateChainAssetLockProofStructure(rawProof);

      expectValidationError(result, InvalidAssetLockProofTransactionHeightError);
      const [error] = result.getErrors();

      expect(error.getProofCoreChainLockedHeight()).to.equal(41);
      expect(error.getTransactionHeight()).to.equal(42);

      expect(stateRepositoryMock.fetchTransaction).to.be.calledOnceWithExactly(
        '6e200d059fb567ba19e92f5c2dcd3dde522fd4e0a50af223752db16158dabb1d',
      );
      expect(validateAssetLockTransactionMock).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformBlockHeader).to.be.calledOnce();
    });
  });

  it('should return valid result', async () => {
    const result = await validateChainAssetLockProofStructure(rawProof);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
    expect(result.getData()).to.deep.equal(publicKeyHash);

    expect(stateRepositoryMock.fetchTransaction).to.be.calledOnceWithExactly(
      '6e200d059fb567ba19e92f5c2dcd3dde522fd4e0a50af223752db16158dabb1d',
    );
    expect(validateAssetLockTransactionMock).to.be.calledOnceWithExactly(
      Buffer.from(rawTransaction, 'hex'),
      0,
    );
    expect(stateRepositoryMock.fetchLatestPlatformBlockHeader).to.be.calledOnce();
  });
});
