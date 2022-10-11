const validateStateTransitionFeeFactory = require('../../../../lib/stateTransition/validation/validateStateTransitionFeeFactory');

const createStateRepositoryMock = require('../../../../lib/test/mocks/createStateRepositoryMock');

const getIdentityFixture = require('../../../../lib/test/fixtures/getIdentityFixture');
const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');
const getIdentityCreateTransitionFixture = require('../../../../lib/test/fixtures/getIdentityCreateTransitionFixture');
const getDocumentTransitionsFixture = require('../../../../lib/test/fixtures/getDocumentTransitionsFixture');
const getIdentityTopUpTransitionFixture = require('../../../../lib/test/fixtures/getIdentityTopUpTransitionFixture');

const DataContractCreateTransition = require('../../../../lib/dataContract/stateTransition/DataContractCreateTransition/DataContractCreateTransition');
const DocumentsBatchTransition = require('../../../../lib/document/stateTransition/DocumentsBatchTransition/DocumentsBatchTransition');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const IdentityBalanceIsNotEnoughError = require('../../../../lib/errors/consensus/fee/BalanceIsNotEnoughError');
const InvalidStateTransitionTypeError = require('../../../../lib/stateTransition/errors/InvalidStateTransitionTypeError');

const { RATIO } = require('../../../../lib/identity/creditsConverter');
const StateTransitionExecutionContext = require('../../../../lib/stateTransition/StateTransitionExecutionContext');

describe('validateStateTransitionFeeFactory', () => {
  let stateRepositoryMock;
  let validateStateTransitionFee;
  let identity;
  let dataContract;
  let calculateStateTransitionFeeMock;
  let fetchAssetLockTransactionOutputMock;

  beforeEach(function beforeEach() {
    identity = getIdentityFixture();

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchIdentity.resolves(identity);

    const output = getIdentityCreateTransitionFixture().getAssetLockProof().getOutput();

    calculateStateTransitionFeeMock = this.sinonSandbox.stub().returns(42);
    fetchAssetLockTransactionOutputMock = this.sinonSandbox.stub().resolves(output);

    validateStateTransitionFee = validateStateTransitionFeeFactory(
      stateRepositoryMock,
      calculateStateTransitionFeeMock,
      fetchAssetLockTransactionOutputMock,
    );

    dataContract = getDataContractFixture();
  });

  describe('DataContractCreateTransition', () => {
    let dataContractCreateTransition;

    beforeEach(() => {
      dataContractCreateTransition = new DataContractCreateTransition({
        dataContract: dataContract.toObject(),
        entropy: dataContract.getEntropy(),
      });
    });

    it('should return invalid result if balance is not enough', async () => {
      identity.balance = 1;

      const result = await validateStateTransitionFee(dataContractCreateTransition);

      expectValidationError(result, IdentityBalanceIsNotEnoughError);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(3000);
      expect(error.getBalance()).to.equal(identity.balance);

      expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
        dataContract.getOwnerId(),
        dataContractCreateTransition.getExecutionContext(),
      );

      expect(calculateStateTransitionFeeMock).to.be.calledOnceWithExactly(
        dataContractCreateTransition,
      );

      expect(fetchAssetLockTransactionOutputMock).to.not.be.called();
    });

    it('should return valid result', async () => {
      identity.balance = 42;

      const result = await validateStateTransitionFee(dataContractCreateTransition);

      expect(result.isValid()).to.be.true();

      expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
        dataContract.getOwnerId(),
        dataContractCreateTransition.getExecutionContext(),
      );

      expect(calculateStateTransitionFeeMock).to.be.calledOnceWithExactly(
        dataContractCreateTransition,
      );

      expect(fetchAssetLockTransactionOutputMock).to.not.be.called();
    });
  });

  describe('DocumentsBatchTransition', () => {
    let documentsBatchTransition;

    beforeEach(() => {
      const documents = getDocumentsFixture(dataContract);

      const documentTransitions = getDocumentTransitionsFixture({
        create: documents,
      });

      documentsBatchTransition = new DocumentsBatchTransition({
        ownerId: getDocumentsFixture.ownerId,
        contractId: dataContract.getId(),
        transitions: documentTransitions.map((t) => t.toObject()),
      }, [dataContract]);
    });

    it('should return invalid result if balance is not enough', async () => {
      identity.balance = 1;

      const result = await validateStateTransitionFee(documentsBatchTransition);

      expectValidationError(result, IdentityBalanceIsNotEnoughError);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(3000);
      expect(error.getBalance()).to.equal(identity.balance);

      expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
        getDocumentsFixture.ownerId,
        documentsBatchTransition.getExecutionContext(),
      );

      expect(calculateStateTransitionFeeMock).to.be.calledOnceWithExactly(
        documentsBatchTransition,
      );

      expect(fetchAssetLockTransactionOutputMock).to.not.be.called();
    });

    it('should return valid result', async () => {
      identity.balance = 42;

      const result = await validateStateTransitionFee(documentsBatchTransition);

      expect(result.isValid()).to.be.true();

      expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
        getDocumentsFixture.ownerId,
        documentsBatchTransition.getExecutionContext(),
      );

      expect(calculateStateTransitionFeeMock).to.be.calledOnceWithExactly(
        documentsBatchTransition,
      );

      expect(fetchAssetLockTransactionOutputMock).to.not.be.called();
    });

    it('should not increase balance on dry run', async () => {
      documentsBatchTransition.getExecutionContext().enableDryRun();

      const result = await validateStateTransitionFee(documentsBatchTransition);

      documentsBatchTransition.getExecutionContext().disableDryRun();

      expect(result.isValid()).to.be.true();

      expect(calculateStateTransitionFeeMock).to.be.not.called();
      expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
        getDocumentsFixture.ownerId,
        documentsBatchTransition.getExecutionContext(),
      );
      expect(fetchAssetLockTransactionOutputMock).to.not.be.called();
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
      calculateStateTransitionFeeMock.returns(outputAmount + 1);

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
      identity.balance = 1;

      calculateStateTransitionFeeMock.returns(outputAmount + 2);

      const result = await validateStateTransitionFee(identityTopUpTransition);

      expectValidationError(result, IdentityBalanceIsNotEnoughError);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(3000);
      expect(error.getBalance()).to.equal(outputAmount + identity.balance);

      expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
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
      identity.balance = 41;

      calculateStateTransitionFeeMock.returns(outputAmount - 1);

      const result = await validateStateTransitionFee(identityTopUpTransition);

      expect(result.isValid()).to.be.true();

      expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
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
      expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
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
