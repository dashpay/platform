const { Transaction, Script, PrivateKey } = require('@dashevo/dashcore-lib');

const { expect } = require('chai');
const getChainAssetLockFixture = require('../../../../../../lib/test/fixtures/getChainAssetLockProofFixture');
const createStateRepositoryMock = require('../../../../../../lib/test/mocks/createStateRepositoryMock');

const { expectJsonSchemaError, expectValidationError } = require('../../../../../../lib/test/expect/expectError');

const { default: loadWasmDpp } = require('../../../../../../dist');

describe.skip('validateChainAssetLockProofStructureFactory', () => {
  let rawProof;
  let stateRepositoryMock;
  let publicKeyHash;
  let rawTransaction;
  let transactionHash;
  let executionContext;
  let validateChainAssetLockProofStructure;

  let StateTransitionExecutionContext;
  let ValidationResult;
  let InvalidAssetLockProofCoreChainHeightError;
  let IdentityAssetLockTransactionIsNotFoundError;
  let InvalidIdentityAssetLockTransactionOutputError;
  let InvalidAssetLockProofTransactionHeightError;
  let ChainAssetLockProofStructureValidator;

  before(async () => {
    ({
      StateTransitionExecutionContext,
      ValidationResult,
      InvalidAssetLockProofCoreChainHeightError,
      IdentityAssetLockTransactionIsNotFoundError,
      InvalidIdentityAssetLockTransactionOutputError,
      InvalidAssetLockProofTransactionHeightError,
      ChainAssetLockProofStructureValidator,
    } = await loadWasmDpp());
  });

  beforeEach(async function beforeEach() {
    rawTransaction = '030000000137feb5676d0851337ea3c9a992496aab7a0b3eee60aeeb9774000b7f4bababa5000000006b483045022100d91557de37645c641b948c6cd03b4ae3791a63a650db3e2fee1dcf5185d1b10402200e8bd410bf516ca61715867666d31e44495428ce5c1090bf2294a829ebcfa4ef0121025c3cc7fbfc52f710c941497fd01876c189171ea227458f501afcb38a297d65b4ffffffff021027000000000000166a14152073ca2300a86b510fa2f123d3ea7da3af68dcf77cb0090a0000001976a914152073ca2300a86b510fa2f123d3ea7da3af68dc88ac00000000';
    transactionHash = '6e200d059fb567ba19e92f5c2dcd3dde522fd4e0a50af223752db16158dabb1d';

    rawProof = getChainAssetLockFixture().toObject();

    // Change endianness of raw txId bytes in outPoint to match expectation of dashcore-rust
    const txId = rawProof.outPoint.slice(0, 32);
    const outputIndex = rawProof.outPoint.slice(32);
    txId.reverse();
    rawProof.outPoint = Buffer.concat([txId, outputIndex]);

    stateRepositoryMock = createStateRepositoryMock(this.sinon);

    stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight.resolves(42);
    stateRepositoryMock.isAssetLockTransactionOutPointAlreadyUsed.resolves(false);

    stateRepositoryMock.fetchTransaction.resolves({
      data: Buffer.from(rawTransaction, 'hex'),
      height: 42,
    });

    executionContext = new StateTransitionExecutionContext();

    publicKeyHash = Buffer.from('152073ca2300a86b510fa2f123d3ea7da3af68dc', 'hex');

    const validator = new ChainAssetLockProofStructureValidator(
      stateRepositoryMock,
    );

    validateChainAssetLockProofStructure = (proof, context) => validator.validate(
      rawProof,
      context,
    );
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawProof.type;

      const result = await validateChainAssetLockProofStructure(
        rawProof,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('type');

      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight).to.not.be.called();
    });

    it('should be equal to 1', async () => {
      rawProof.type = -1;

      const result = await validateChainAssetLockProofStructure(
        rawProof,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/type');
      expect(error.getKeyword()).to.equal('const');

      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight).to.not.be.called();
    });
  });

  describe('coreChainLockedHeight', () => {
    it('should be preset', async () => {
      delete rawProof.coreChainLockedHeight;

      const result = await validateChainAssetLockProofStructure(
        rawProof,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('coreChainLockedHeight');

      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight).to.not.be.called();
    });

    it('should be an integer', async () => {
      rawProof.coreChainLockedHeight = 1.5;

      const result = await validateChainAssetLockProofStructure(
        rawProof,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/coreChainLockedHeight');
      expect(error.getKeyword()).to.equal('type');

      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight).to.not.be.called();
    });

    it('should be a number', async () => {
      rawProof.coreChainLockedHeight = '42';

      const result = await validateChainAssetLockProofStructure(
        rawProof,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/coreChainLockedHeight');
      expect(error.getKeyword()).to.equal('type');

      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight).to.not.be.called();
    });

    it('should be greater than 0', async () => {
      rawProof.coreChainLockedHeight = 0;

      const result = await validateChainAssetLockProofStructure(
        rawProof,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/coreChainLockedHeight');
      expect(error.getKeyword()).to.equal('minimum');

      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight).to.not.be.called();
    });

    it('should be less than 4294967296', async () => {
      rawProof.coreChainLockedHeight = 4294967296;

      const result = await validateChainAssetLockProofStructure(
        rawProof,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/coreChainLockedHeight');
      expect(error.getKeyword()).to.equal('maximum');

      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight).to.not.be.called();
    });

    it('should be less or equal to consensus core height', async () => {
      stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight.resolves(41);

      const result = await validateChainAssetLockProofStructure(
        rawProof,
        executionContext,
      );

      expectValidationError(result, InvalidAssetLockProofCoreChainHeightError);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(1035);
      expect(error.getProofCoreChainLockedHeight()).to.equal(42);
      expect(error.getCurrentCoreChainLockedHeight()).to.equal(41);
    });
  });

  describe('outPoint', () => {
    it('should be present', async () => {
      delete rawProof.outPoint;

      const result = await validateChainAssetLockProofStructure(
        rawProof,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('outPoint');

      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight).to.not.be.called();
    });

    it('should be a byte array', async () => {
      rawProof.outPoint = new Array(36).fill('string');

      const result = await validateChainAssetLockProofStructure(
        rawProof,
        executionContext,
      );

      await expectJsonSchemaError(result, 36);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/outPoint/0');
      expect(error.getKeyword()).to.equal('type');

      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight).to.not.be.called();
    });

    it('should not be shorter than 36 bytes', async () => {
      rawProof.outPoint = Buffer.alloc(35);

      const result = await validateChainAssetLockProofStructure(
        rawProof,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/outPoint');
      expect(error.getKeyword()).to.equal('minItems');

      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight).to.not.be.called();
    });

    it('should not be longer than 36 bytes', async () => {
      rawProof.outPoint = Buffer.alloc(37);

      const result = await validateChainAssetLockProofStructure(
        rawProof,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/outPoint');
      expect(error.getKeyword()).to.equal('maxItems');

      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
      expect(stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight).to.not.be.called();
    });

    it('should point to existing transaction', async function () {
      stateRepositoryMock.fetchTransaction.resolves(null);

      const result = await validateChainAssetLockProofStructure(
        rawProof,
        executionContext,
      );

      await expectValidationError(result, IdentityAssetLockTransactionIsNotFoundError);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(1032);
      expect(error.getTransactionId()).to.deep.equal(
        Buffer.from(transactionHash, 'hex'),
      );

      expect(stateRepositoryMock.fetchTransaction).to.be.calledOnceWithExactly(
        transactionHash,
        this.sinon.match.instanceOf(StateTransitionExecutionContext),
      );
      expect(stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight).to.be.calledOnce();
    });

    it('should point to valid transaction', async () => {
      // Validator expects asset lock proof TX to have OP_RETURN in it's first output, break it
      const parsedTx = new Transaction(Buffer.from(rawTransaction, 'hex'));
      const fromAddress = new PrivateKey().toAddress();
      parsedTx.outputs[0].setScript(Script.buildPublicKeyHashOut(fromAddress).toString());

      stateRepositoryMock.fetchTransaction.resolves({
        data: parsedTx.toBuffer(),
        height: 42,
      });

      const result = await validateChainAssetLockProofStructure(
        rawProof,
        executionContext,
      );

      await expectValidationError(
        result,
        InvalidIdentityAssetLockTransactionOutputError,
      );
    });

    it('should point to transaction from block lower than core chain locked height', async function () {
      rawProof.coreChainLockedHeight = 41;
      stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight.resolves(41);

      const result = await validateChainAssetLockProofStructure(
        rawProof,
        executionContext,
      );

      await expectValidationError(result, InvalidAssetLockProofTransactionHeightError);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(1036);
      expect(error.getProofCoreChainLockedHeight()).to.equal(41);
      expect(error.getTransactionHeight()).to.equal(42);

      expect(stateRepositoryMock.fetchTransaction).to.be.calledOnceWithExactly(
        transactionHash,
        this.sinon.match.instanceOf(StateTransitionExecutionContext),
      );
      expect(stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight).to.be.calledOnce();
    });
  });

  it('should return valid result', async function () {
    const result = await validateChainAssetLockProofStructure(
      rawProof,
      executionContext,
    );

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
    expect(result.getData()).to.deep.equal(publicKeyHash);

    expect(stateRepositoryMock.fetchTransaction).to.be.calledOnceWithExactly(
      transactionHash,
      this.sinon.match.instanceOf(StateTransitionExecutionContext),
    );

    expect(stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight).to.be.calledOnce();
  });
});
