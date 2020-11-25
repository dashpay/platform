const validateStateTransitionFeeFactory = require('../../../../lib/stateTransition/validation/validateStateTransitionFeeFactory');

const createStateRepositoryMock = require('../../../../lib/test/mocks/createStateRepositoryMock');

const getIdentityFixture = require('../../../../lib/test/fixtures/getIdentityFixture');
const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');
const getIdentityCreateTransitionFixture = require('../../../../lib/test/fixtures/getIdentityCreateTransitionFixture');
const getDocumentTransitionsFixture = require('../../../../lib/test/fixtures/getDocumentTransitionsFixture');
const getIdentityTopUpTransitionFixture = require('../../../../lib/test/fixtures/getIdentityTopUpTransitionFixture');

const DataContractCreateTransition = require('../../../../lib/dataContract/stateTransition/DataContractCreateTransition');
const DocumentsBatchTransition = require('../../../../lib/document/stateTransition/DocumentsBatchTransition');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const IdentityBalanceIsNotEnoughError = require('../../../../lib/errors/BalanceIsNotEnoughError');
const InvalidStateTransitionTypeError = require('../../../../lib/errors/InvalidStateTransitionTypeError');

const { RATIO } = require('../../../../lib/identity/creditsConverter');

describe('validateStateTransitionFeeFactory', () => {
  let stateRepositoryMock;
  let validateStateTransitionFee;
  let identity;
  let dataContract;
  let calculateStateTransitionFeeMock;

  beforeEach(function beforeEach() {
    identity = getIdentityFixture();

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchIdentity.resolves(identity);

    calculateStateTransitionFeeMock = this.sinonSandbox.stub().returns(2);

    validateStateTransitionFee = validateStateTransitionFeeFactory(
      stateRepositoryMock,
      calculateStateTransitionFeeMock,
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

      expect(error.getBalance()).to.equal(identity.balance);

      expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
        dataContract.getOwnerId(),
      );

      expect(calculateStateTransitionFeeMock).to.be.calledOnceWithExactly(
        dataContractCreateTransition,
      );
    });

    it('should return valid result', async () => {
      identity.balance = 2;

      const result = await validateStateTransitionFee(dataContractCreateTransition);

      expect(result.isValid()).to.be.true();

      expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
        dataContract.getOwnerId(),
      );

      expect(calculateStateTransitionFeeMock).to.be.calledOnceWithExactly(
        dataContractCreateTransition,
      );
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

      expect(error.getBalance()).to.equal(identity.balance);

      expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
        getDocumentsFixture.ownerId,
      );

      expect(calculateStateTransitionFeeMock).to.be.calledOnceWithExactly(
        documentsBatchTransition,
      );
    });

    it('should return valid result', async () => {
      identity.balance = 3;

      const result = await validateStateTransitionFee(documentsBatchTransition);

      expect(result.isValid()).to.be.true();

      expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
        getDocumentsFixture.ownerId,
      );

      expect(calculateStateTransitionFeeMock).to.be.calledOnceWithExactly(
        documentsBatchTransition,
      );
    });
  });

  describe('IdentityCreateStateTransition', () => {
    let identityCreateTransition;
    let outputAmount;

    beforeEach(() => {
      identityCreateTransition = getIdentityCreateTransitionFixture();

      const { satoshis } = identityCreateTransition.getAssetLock().getOutput();

      outputAmount = satoshis * RATIO;
    });

    it('should return invalid result if asset lock output amount is not enough', async () => {
      calculateStateTransitionFeeMock.returns(outputAmount + 1);

      const result = await validateStateTransitionFee(identityCreateTransition);

      expectValidationError(result, IdentityBalanceIsNotEnoughError);

      const [error] = result.getErrors();

      expect(error.getBalance()).to.equal(outputAmount);

      expect(stateRepositoryMock.fetchIdentity).to.be.not.called();

      expect(calculateStateTransitionFeeMock).to.be.calledOnceWithExactly(
        identityCreateTransition,
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
    });
  });

  describe('IdentityTopUpTransition', () => {
    let identityTopUpTransition;
    let outputAmount;

    beforeEach(() => {
      identityTopUpTransition = getIdentityTopUpTransitionFixture();

      const { satoshis } = identityTopUpTransition.getAssetLock().getOutput();

      outputAmount = satoshis * RATIO;
    });

    it('should return invalid result if sum of balance and asset lock output amount is not enough', async () => {
      identity.balance = 1;

      calculateStateTransitionFeeMock.returns(outputAmount + 2);

      const result = await validateStateTransitionFee(identityTopUpTransition);

      expectValidationError(result, IdentityBalanceIsNotEnoughError);

      const [error] = result.getErrors();

      expect(error.getBalance()).to.equal(outputAmount + identity.balance);

      expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
        identityTopUpTransition.getIdentityId(),
      );

      expect(calculateStateTransitionFeeMock).to.be.calledOnceWithExactly(
        identityTopUpTransition,
      );
    });

    it('should return valid result', async () => {
      identity.balance = 1;

      calculateStateTransitionFeeMock.returns(outputAmount - 1);

      const result = await validateStateTransitionFee(identityTopUpTransition);

      expect(result.isValid()).to.be.true();

      expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
        identityTopUpTransition.getIdentityId(),
      );

      expect(calculateStateTransitionFeeMock).to.be.calledOnceWithExactly(
        identityTopUpTransition,
      );
    });
  });

  it('should throw InvalidStateTransitionTypeError on invalid State Transition', async function it() {
    const rawStateTransitionMock = {
      data: 'sample data',
    };

    const stateTransitionMock = {
      getType: this.sinonSandbox.stub().returns(-1),
      toBuffer: this.sinonSandbox.stub().returns(Buffer.alloc(0)),
      toObject: this.sinonSandbox.stub().returns(rawStateTransitionMock),
    };

    try {
      await validateStateTransitionFee(stateTransitionMock);

      expect.fail('should throw InvalidStateTransitionTypeError');
    } catch (error) {
      expect(error).to.be.an.instanceOf(InvalidStateTransitionTypeError);
      expect(error.getRawStateTransition()).to.equal(rawStateTransitionMock);

      expect(calculateStateTransitionFeeMock).to.be.calledOnceWithExactly(stateTransitionMock);
      expect(stateRepositoryMock.fetchIdentity).to.not.be.called();
    }
  });
});
