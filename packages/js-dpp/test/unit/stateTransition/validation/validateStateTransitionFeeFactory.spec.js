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
  let documents;
  let identityCreateST;
  let identityTopUpST;
  let getLockedTransactionOutputMock;
  let output;

  beforeEach(function beforeEach() {
    identityCreateST = getIdentityCreateTransitionFixture();
    identityTopUpST = getIdentityTopUpTransitionFixture();

    const stSize = Buffer.byteLength(identityCreateST.toBuffer({ skipSignature: true }));

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
    documents = getDocumentsFixture(dataContract);
  });

  it('should return invalid result if balance is not enough', async () => {
    const dataContractCreateTransition = new DataContractCreateTransition({
      dataContract: dataContract.toObject(),
      entropy: dataContract.getEntropy(),
    });

    const serializedData = dataContractCreateTransition.toBuffer({ skipSignature: true });
    identity.balance = Buffer.byteLength(serializedData) - 1;

    const result = await validateStateTransitionFee(dataContractCreateTransition);

    expectValidationError(result, IdentityBalanceIsNotEnoughError);

    const [error] = result.getErrors();

    expect(error.getBalance()).to.equal(identity.balance);
  });

  it('should return valid result for DataContractCreateTransition', async () => {
    const dataContractCreateTransition = new DataContractCreateTransition({
      dataContract: dataContract.toObject(),
      entropy: dataContract.getEntropy(),
    });

    const serializedData = dataContractCreateTransition.toBuffer({ skipSignature: true });
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
      transitions: documentTransitions.map((t) => t.toObject()),
    }, [dataContract]);
    identity.balance = Buffer.byteLength(stateTransition.toBuffer({ skipSignature: true }));

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
      toBuffer: this.sinonSandbox.stub().returns(Buffer.alloc(0)),
      toObject: this.sinonSandbox.stub().returns(rawStateTransitionMock),
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
    expect(stateTransitionMock.toBuffer).to.be.calledOnce();
  });
});
