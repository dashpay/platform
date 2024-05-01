const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const createStateRepositoryMock = require('../../../../lib/test/mocks/createStateRepositoryMock');
const getBlsMock = require('../../../../lib/test/mocks/getBlsAdapterMock');

const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');
const getIdentityCreateTransitionFixture = require('../../../../lib/test/fixtures/getIdentityCreateTransitionFixture');
const getIdentityTopUpTransitionFixture = require('../../../../lib/test/fixtures/getIdentityTopUpTransitionFixture');

let {
  StateTransitionFeeValidator,
  DashPlatformProtocol,
  IdentityBalanceIsNotEnoughError,
  StateTransitionExecutionContext,
  PreCalculatedOperation,
  getCreditsConversionRatio,
} = require('../../../..');
const { default: loadWasmDpp } = require('../../../..');

describe.skip('validateStateTransitionFeeFactory', () => {
  let stateRepositoryMock;
  let validateStateTransitionFee;
  let dataContract;
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
      getCreditsConversionRatio,
    } = await loadWasmDpp());
    stateRepositoryMock = createStateRepositoryMock(this.sinon);
    stateRepositoryMock.fetchDataContract.resolves(undefined);

    dpp = new DashPlatformProtocol(getBlsMock(), stateRepositoryMock);

    const validator = new StateTransitionFeeValidator(stateRepositoryMock);

    executionContext = new StateTransitionExecutionContext();
    executionContext.disableDryRun();
    validateStateTransitionFee = (st) => validator.validate(st, executionContext);

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
      stateRepositoryMock.fetchIdentityBalance.resolves(1);

      const result = await validateStateTransitionFee(dataContractCreateTransition);

      await expectValidationError(result, IdentityBalanceIsNotEnoughError);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(3000);
      expect(error.getBalance()).to.equal(1);

      expect(stateRepositoryMock.fetchIdentityBalance).to.be.calledOnce();
      expect(
        stateRepositoryMock.fetchIdentityBalance.getCall(0).args[0].toBuffer(),
      ).to.be.deep.equal(dataContractOwnerId);
    });

    it('should return valid result', async () => {
      stateRepositoryMock.fetchIdentityBalance.resolves(42);

      executionContext.addOperation(new PreCalculatedOperation(0, 42, []));

      const result = await validateStateTransitionFee(dataContractCreateTransition);

      expect(result.isValid()).to.be.true();

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

      const result = await validateStateTransitionFee(documentsBatchTransition);

      executionContext.disableDryRun();

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

    beforeEach(async () => {
      identityCreateTransition = await getIdentityCreateTransitionFixture();

      const { satoshis } = identityCreateTransition
        .getAssetLockProof()
        .getOutput();

      outputAmount = satoshis * getCreditsConversionRatio();
    });

    it('should return invalid result if asset lock output amount is not enough', async () => {
      executionContext.addOperation(new PreCalculatedOperation(0, outputAmount + 1, []));

      const result = await validateStateTransitionFee(identityCreateTransition);

      await expectValidationError(result, IdentityBalanceIsNotEnoughError);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(3000);
      expect(error.getBalance()).to.equal(outputAmount);

      expect(stateRepositoryMock.fetchIdentity).to.be.not.called();
    });

    it('should return valid result', async () => {
      executionContext.addOperation(new PreCalculatedOperation(0, outputAmount, []));

      const result = await validateStateTransitionFee(identityCreateTransition);

      expect(result.isValid()).to.be.true();

      expect(stateRepositoryMock.fetchIdentity).to.be.not.called();
    });

    it('should not increase balance on dry run', async () => {
      executionContext.enableDryRun();
      executionContext.addOperation(new PreCalculatedOperation(0, outputAmount + 1, []));

      const result = await validateStateTransitionFee(identityCreateTransition);

      executionContext.disableDryRun();

      expect(result.isValid()).to.be.true();
    });
  });

  describe('IdentityTopUpTransition', () => {
    let identityTopUpTransition;
    let outputAmount;
    let identityId;

    beforeEach(async () => {
      identityTopUpTransition = await getIdentityTopUpTransitionFixture();
      identityId = identityTopUpTransition.getIdentityId().toBuffer();

      const { satoshis } = identityTopUpTransition
        .getAssetLockProof()
        .getOutput();

      outputAmount = satoshis * getCreditsConversionRatio();
    });

    it('should return invalid result if sum of balance and asset lock output amount is not enough', async () => {
      stateRepositoryMock.fetchIdentityBalanceWithDebt.resolves(1);

      executionContext.addOperation(new PreCalculatedOperation(0, outputAmount + 2, []));

      const result = await validateStateTransitionFee(identityTopUpTransition);

      await expectValidationError(result, IdentityBalanceIsNotEnoughError);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(3000);
      expect(error.getBalance()).to.equal(outputAmount + 1);

      expect(stateRepositoryMock.fetchIdentityBalanceWithDebt).to.be.calledOnce();
      expect(
        stateRepositoryMock.fetchIdentityBalanceWithDebt.getCall(0).args[0].toBuffer(),
      ).to.be.deep.equal(identityId);
    });

    it('should return valid result', async () => {
      stateRepositoryMock.fetchIdentityBalanceWithDebt.resolves(41);

      executionContext.addOperation(new PreCalculatedOperation(0, outputAmount - 1, []));

      const result = await validateStateTransitionFee(identityTopUpTransition);

      expect(result.isValid()).to.be.true();

      expect(stateRepositoryMock.fetchIdentityBalanceWithDebt).to.be.calledOnce();
      expect(
        stateRepositoryMock.fetchIdentityBalanceWithDebt.getCall(0).args[0].toBuffer(),
      ).to.be.deep.equal(identityId);
    });

    it('should not increase balance on dry run', async () => {
      stateRepositoryMock.fetchIdentityBalanceWithDebt.resolves(1);

      executionContext.enableDryRun();
      executionContext.addOperation(new PreCalculatedOperation(0, outputAmount + 42, []));

      const result = await validateStateTransitionFee(identityTopUpTransition);

      executionContext.disableDryRun();

      expect(result.isValid()).to.be.true();

      expect(stateRepositoryMock.fetchIdentityBalanceWithDebt).to.be.calledOnce();
      expect(
        stateRepositoryMock.fetchIdentityBalanceWithDebt.getCall(0).args[0].toBuffer(),
      ).to.be.deep.equal(identityId);
    });
  });
});
