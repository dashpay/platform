const getIdentityCreateTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityCreateTransitionFixture');
const getIdentityTopUpTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityTopUpTransitionFixture');

const InvalidStateTransitionTypeError = require('@dashevo/dpp/lib/stateTransition/errors/InvalidStateTransitionTypeError');

const { RATIO } = require('@dashevo/dpp/lib/identity/creditsConverter');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');
const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');

const createStateRepositoryMock = require('../../../../lib/test/mocks/createStateRepositoryMock');
const getBlsMock = require('../../../../lib/test/mocks/getBlsAdapterMock');

const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');

let {
  StateTransitionFeeValidator,
  DashPlatformProtocol,
  IdentityBalanceIsNotEnoughError,
  StateTransitionExecutionContext,
  PreCalculatedOperation,
} = require('../../../..');
const { default: loadWasmDpp } = require('../../../..');

describe('validateStateTransitionFeeFactory', () => {
  let stateRepositoryMock;
  let validateStateTransitionFee;
  let dataContract;
  let calculateStateTransitionFeeMock;
  let fetchAssetLockTransactionOutputMock;
  let dpp;
  let dataContractOwnerId;
  let executionContext;

  beforeEach(async function beforeEach() {
    ({
      StateTransitionFeeValidator,
      IdentityBalanceIsNotEnoughError,
      DashPlatformProtocol,
      StateTransitionExecutionContext,
      PreCalculatedOperation,
    } = await loadWasmDpp());
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchDataContract.resolves(undefined);

    const output = getIdentityCreateTransitionFixture().getAssetLockProof().getOutput();

    calculateStateTransitionFeeMock = this.sinonSandbox.stub().returns({
      desiredAmount: 42,
    });

    dpp = new DashPlatformProtocol(getBlsMock(), stateRepositoryMock);
    fetchAssetLockTransactionOutputMock = this.sinonSandbox.stub().resolves(output);

    const validator = new StateTransitionFeeValidator(stateRepositoryMock);

    validateStateTransitionFee = (st) => validator.validate(st);

    executionContext = new StateTransitionExecutionContext();
    executionContext.disableDryRun();

    dataContract = await getDataContractFixture();
  });

  describe('DataContractCreateTransition', () => {
    let dataContractCreateTransition;

    beforeEach(() => {
      dataContractOwnerId = dataContract.getOwnerId().toBuffer();
      dataContractCreateTransition = dpp.dataContract
        .createDataContractCreateTransition(dataContract);
    });

    it('should return invalid result if balance is not enough', async () => {
      executionContext.addOperation(new PreCalculatedOperation(0, 42, []));
      dataContractCreateTransition.setExecutionContext(executionContext);
      stateRepositoryMock.fetchIdentityBalance.resolves(1);

      const result = await validateStateTransitionFee(dataContractCreateTransition);

      await expectValidationError(result, IdentityBalanceIsNotEnoughError);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(3000);
      //
      expect(error.getBalance()).to.equal(1);

      expect(stateRepositoryMock.fetchIdentityBalance).to.be.calledOnce();
      expect(
        stateRepositoryMock.fetchIdentityBalance.getCall(0).args[0].toBuffer(),
      ).to.be.deep.equal(dataContractOwnerId);
    });

    it('should return valid result', async () => {
      stateRepositoryMock.fetchIdentityBalance.resolves(42);

      executionContext.addOperation(new PreCalculatedOperation(0, 42, []));
      dataContractCreateTransition.setExecutionContext(executionContext);

      const result = await validateStateTransitionFee(dataContractCreateTransition);

      expect(result.isValid()).to.be.true();
      console.log(result.getErrors()[0]);

      expect(stateRepositoryMock.fetchIdentityBalance).to.be.calledOnce();
      expect(
        stateRepositoryMock.fetchIdentityBalance.getCall(0).args[0].toBuffer(),
      ).to.be.deep.equal(dataContractOwnerId);
    });
  });

  describe('DocumentsBatchTransition', () => {
    let documentsBatchTransition;
    let ownerId;

    beforeEach(async () => {
      documentsBatchTransition = await dpp.document.createStateTransition({
        create: await getDocumentsFixture(),
      });

      ownerId = documentsBatchTransition.getOwnerId().toBuffer();
    });

    it('should return invalid result if balance is not enough', async () => {
      stateRepositoryMock.fetchIdentityBalance.resolves(1);

      executionContext.addOperation(new PreCalculatedOperation(0, 42, []));
      documentsBatchTransition.setExecutionContext(executionContext);

      const result = await validateStateTransitionFee(documentsBatchTransition);

      await expectValidationError(result, IdentityBalanceIsNotEnoughError);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(3000);
      expect(error.getBalance()).to.equal(1);

      expect(stateRepositoryMock.fetchIdentityBalance).to.be.calledOnce();
      expect(
        stateRepositoryMock.fetchIdentityBalance.getCall(0).args[0].toBuffer(),
      ).to.be.deep.equal(ownerId);
    });

    it('should return valid result', async () => {
      stateRepositoryMock.fetchIdentityBalance.resolves(42);

      executionContext.addOperation(new PreCalculatedOperation(0, 42, []));
      documentsBatchTransition.setExecutionContext(executionContext);

      const result = await validateStateTransitionFee(documentsBatchTransition);

      expect(result.isValid()).to.be.true();

      expect(stateRepositoryMock.fetchIdentityBalance).to.be.calledOnce();
      expect(
        stateRepositoryMock.fetchIdentityBalance.getCall(0).args[0].toBuffer(),
      ).to.be.deep.equal(ownerId);
    });

    it('should not increase balance on dry run', async () => {
      stateRepositoryMock.fetchIdentityBalance.resolves(1);

      executionContext.enableDryRun();

      executionContext.addOperation(new PreCalculatedOperation(0, 42, []));
      documentsBatchTransition.setExecutionContext(executionContext);

      const result = await validateStateTransitionFee(documentsBatchTransition);

      documentsBatchTransition.getExecutionContext().disableDryRun();

      expect(result.isValid()).to.be.true();

      expect(stateRepositoryMock.fetchIdentityBalance).to.be.calledOnce();
      expect(
        stateRepositoryMock.fetchIdentityBalance.getCall(0).args[0].toBuffer(),
      ).to.be.deep.equal(ownerId);
    });
  });

  describe('IdentityCreateStateTransition', () => {
    let identityCreateTransition;
    let outputAmount;

    beforeEach(() => {
      identityCreateTransition = getIdentityCreateTransitionFixture();

      const { satoshis } = identityCreateTransition
        .getAssetLockProof()
        .getOutput();

      outputAmount = satoshis * RATIO;
    });

    it('should return invalid result if asset lock output amount is not enough', async () => {
      calculateStateTransitionFeeMock.returns({ desiredAmount: outputAmount + 1 });

      const result = await validateStateTransitionFee(identityCreateTransition);

      expectValidationError(result, IdentityBalanceIsNotEnoughError);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(3000);
      expect(error.getBalance()).to.equal(outputAmount);

      expect(stateRepositoryMock.fetchIdentity).to.be.not.called();

      expect(calculateStateTransitionFeeMock).to.be.calledOnceWithExactly(
        identityCreateTransition,
      );

      expect(fetchAssetLockTransactionOutputMock).to.be.calledOnceWithExactly(
        identityCreateTransition.getAssetLockProof(),
        identityCreateTransition.getExecutionContext(),
      );
    });

    it('should return valid result', async () => {
      calculateStateTransitionFeeMock.returns(outputAmount);

      const result = await validateStateTransitionFee(identityCreateTransition);

      expect(result.isValid()).to.be.true();

      expect(stateRepositoryMock.fetchIdentity).to.be.not.called();

      expect(calculateStateTransitionFeeMock).to.be.calledOnceWithExactly(
        identityCreateTransition,
      );

      expect(fetchAssetLockTransactionOutputMock).to.be.calledOnceWithExactly(
        identityCreateTransition.getAssetLockProof(),
        identityCreateTransition.getExecutionContext(),
      );
    });

    it('should not increase balance on dry run', async () => {
      identityCreateTransition.getExecutionContext().enableDryRun();

      const result = await validateStateTransitionFee(identityCreateTransition);

      identityCreateTransition.getExecutionContext().disableDryRun();

      expect(result.isValid()).to.be.true();

      expect(calculateStateTransitionFeeMock).to.be.not.called();
      expect(fetchAssetLockTransactionOutputMock).to.be.calledOnceWithExactly(
        identityCreateTransition.getAssetLockProof(),
        identityCreateTransition.getExecutionContext(),
      );
    });
  });

  describe('IdentityTopUpTransition', () => {
    let identityTopUpTransition;
    let outputAmount;

    beforeEach(() => {
      identityTopUpTransition = getIdentityTopUpTransitionFixture();

      const { satoshis } = identityTopUpTransition
        .getAssetLockProof()
        .getOutput();

      outputAmount = satoshis * RATIO;
    });

    it('should return invalid result if sum of balance and asset lock output amount is not enough', async () => {
      stateRepositoryMock.fetchIdentityBalanceWithDebt.resolves(1);

      calculateStateTransitionFeeMock.returns({ desiredAmount: outputAmount + 2 });

      const result = await validateStateTransitionFee(identityTopUpTransition);

      expectValidationError(result, IdentityBalanceIsNotEnoughError);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(3000);
      expect(error.getBalance()).to.equal(outputAmount + 1);

      expect(stateRepositoryMock.fetchIdentityBalanceWithDebt).to.be.calledOnceWithExactly(
        identityTopUpTransition.getIdentityId(),
        identityTopUpTransition.getExecutionContext(),
      );

      expect(calculateStateTransitionFeeMock).to.be.calledOnceWithExactly(
        identityTopUpTransition,
      );

      expect(fetchAssetLockTransactionOutputMock).to.be.calledOnceWithExactly(
        identityTopUpTransition.getAssetLockProof(),
        identityTopUpTransition.getExecutionContext(),
      );
    });

    it('should return valid result', async () => {
      stateRepositoryMock.fetchIdentityBalanceWithDebt.resolves(41);

      calculateStateTransitionFeeMock.returns(outputAmount - 1);

      const result = await validateStateTransitionFee(identityTopUpTransition);

      expect(result.isValid()).to.be.true();

      expect(stateRepositoryMock.fetchIdentityBalanceWithDebt).to.be.calledOnceWithExactly(
        identityTopUpTransition.getIdentityId(),
        identityTopUpTransition.getExecutionContext(),
      );

      expect(calculateStateTransitionFeeMock).to.be.calledOnceWithExactly(
        identityTopUpTransition,
      );

      expect(fetchAssetLockTransactionOutputMock).to.be.calledOnceWithExactly(
        identityTopUpTransition.getAssetLockProof(),
        identityTopUpTransition.getExecutionContext(),
      );
    });

    it('should not increase balance on dry run', async () => {
      identityTopUpTransition.getExecutionContext().enableDryRun();

      const result = await validateStateTransitionFee(identityTopUpTransition);

      identityTopUpTransition.getExecutionContext().disableDryRun();

      expect(result.isValid()).to.be.true();

      expect(calculateStateTransitionFeeMock).to.be.not.called();
      expect(fetchAssetLockTransactionOutputMock).to.be.calledOnceWithExactly(
        identityTopUpTransition.getAssetLockProof(),
        identityTopUpTransition.getExecutionContext(),
      );
      expect(stateRepositoryMock.fetchIdentityBalanceWithDebt).to.be.calledOnceWithExactly(
        identityTopUpTransition.getIdentityId(),
        identityTopUpTransition.getExecutionContext(),
      );
    });
  });

  it('should throw InvalidStateTransitionTypeError on invalid State Transition', async function it() {
    const rawStateTransitionMock = {
      data: 'sample data',
      type: -1,
    };

    const stateTransitionMock = {
      getType: this.sinonSandbox.stub().returns(rawStateTransitionMock.type),
      toBuffer: this.sinonSandbox.stub().returns(Buffer.alloc(0)),
      toObject: this.sinonSandbox.stub().returns(rawStateTransitionMock),
      getExecutionContext: this.sinonSandbox.stub().returns(new StateTransitionExecutionContext()),
    };

    try {
      await validateStateTransitionFee(stateTransitionMock);

      expect.fail('should throw InvalidStateTransitionTypeError');
    } catch (error) {
      expect(error).to.be.an.instanceOf(InvalidStateTransitionTypeError);
      expect(error.getType()).to.equal(rawStateTransitionMock.type);

      expect(calculateStateTransitionFeeMock).to.not.be.called();
      expect(stateRepositoryMock.fetchIdentity).to.not.be.called();

      expect(fetchAssetLockTransactionOutputMock).to.not.be.called();
    }
  });
});
