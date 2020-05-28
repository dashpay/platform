const validateStateTransitionFeeFactory = require('../../../../lib/stateTransition/validation/validateStateTransitionFeeFactory');

const createStateRepositoryMock = require('../../../../lib/test/mocks/createStateRepositoryMock');

const getIdentityFixture = require('../../../../lib/test/fixtures/getIdentityFixture');
const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');
const getIdentityCreateSTFixture = require('../../../../lib/test/fixtures/getIdentityCreateSTFixture');
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
  let documents;
  let identityCreateST;
  let identityTopUpST;
  let getLockedTransactionOutputMock;
  let output;

  beforeEach(function beforeEach() {
    identityCreateST = getIdentityCreateSTFixture();
    identityTopUpST = getIdentityTopUpTransitionFixture();

    const stSize = Buffer.byteLength(identityCreateST.serialize({ skipSignature: true }));

    output = {
      satoshis: Math.ceil(stSize / RATIO),
    };

    getLockedTransactionOutputMock = this.sinonSandbox.stub().resolves(output);
    identity = getIdentityFixture();
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchIdentity.resolves(identity);
    validateStateTransitionFee = validateStateTransitionFeeFactory(
      stateRepositoryMock,
      getLockedTransactionOutputMock,
    );
    dataContract = getDataContractFixture();
    documents = getDocumentsFixture();
  });

  it('should return invalid result if balance is not enough', async () => {
    const dataContractCreateTransition = new DataContractCreateTransition({
      dataContract: dataContract.toJSON(),
      entropy: dataContract.getEntropy(),
    });

    const serializedData = dataContractCreateTransition.serialize({ skipSignature: true });
    identity.balance = Buffer.byteLength(serializedData) - 1;

    const result = await validateStateTransitionFee(dataContractCreateTransition);

    expectValidationError(result, IdentityBalanceIsNotEnoughError);

    const [error] = result.getErrors();

    expect(error.getBalance()).to.equal(identity.balance);
  });

  it('should return valid result for DataContractCreateTransition', async () => {
    const dataContractCreateTransition = new DataContractCreateTransition({
      dataContract: dataContract.toJSON(),
      entropy: dataContract.getEntropy(),
    });

    const serializedData = dataContractCreateTransition.serialize({ skipSignature: true });
    identity.balance = Buffer.byteLength(serializedData);

    const result = await validateStateTransitionFee(dataContractCreateTransition);

    expect(result.isValid()).to.be.true();
    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
      dataContract.getOwnerId(),
    );
    expect(getLockedTransactionOutputMock).to.be.not.called();
  });

  it('should return valid result for DocumentsBatchTransition', async () => {
    const documentTransitions = getDocumentTransitionsFixture({
      create: documents,
    });

    const stateTransition = new DocumentsBatchTransition({
      ownerId: getDocumentsFixture.ownerId,
      contractId: dataContract.getId(),
      transitions: documentTransitions.map((t) => t.toJSON()),
    });
    identity.balance = Buffer.byteLength(stateTransition.serialize({ skipSignature: true }));

    const result = await validateStateTransitionFee(stateTransition);

    expect(result.isValid()).to.be.true();
    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
      getDocumentsFixture.ownerId,
    );
    expect(getLockedTransactionOutputMock).to.be.not.called();
  });

  it('should return valid result for IdentityCreateStateTransition', async () => {
    const result = await validateStateTransitionFee(identityCreateST);

    expect(result.isValid()).to.be.true();
    expect(getLockedTransactionOutputMock).to.be.calledOnceWithExactly(
      identityCreateST.getLockedOutPoint(),
    );
    expect(stateRepositoryMock.fetchIdentity).to.be.not.called();
  });

  it('should return valid result for IdentityTopUpTransition', async () => {
    const result = await validateStateTransitionFee(identityTopUpST);

    expect(result.isValid()).to.be.true();
    expect(getLockedTransactionOutputMock).to.be.calledOnceWithExactly(
      identityTopUpST.getLockedOutPoint(),
    );
    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
      identityTopUpST.getIdentityId(),
    );
  });

  it('should throw InvalidStateTransitionTypeError on invalid State Transition', async function it() {
    const rawStateTransitionMock = {
      data: 'sample data',
    };

    const stateTransitionMock = {
      getType: this.sinonSandbox.stub().returns(-1),
      serialize: this.sinonSandbox.stub().returns(Buffer.alloc(0)),
      toJSON: this.sinonSandbox.stub().returns(rawStateTransitionMock),
    };
    identity.balance = 0;

    try {
      await validateStateTransitionFee(stateTransitionMock);

      expect.fail('should throw InvalidStateTransitionTypeError');
    } catch (error) {
      expect(error).to.be.an.instanceOf(InvalidStateTransitionTypeError);
      expect(error.getRawStateTransition()).to.equal(rawStateTransitionMock);
    }

    expect(stateTransitionMock.getType).to.be.calledOnce();
    expect(stateTransitionMock.serialize).to.be.calledOnce();
  });
});
