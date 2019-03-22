const { Transaction, PrivateKey } = require('@dashevo/dashcore-lib');

const STPacket = require('../../../../lib/stPacket/STPacket');

const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');
const getContractFixture = require('../../../../lib/test/fixtures/getContractFixture');

const createDataProviderMock = require('../../../../lib/test/mocks/createDataProviderMock');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const verifySTPacketFactory = require('../../../../lib/stPacket/verification/verifySTPacketFactory');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

const UserNotFoundError = require('../../../../lib/errors/UserNotFoundError');
const UnconfirmedUserError = require('../../../../lib/errors/UnconfirmedUserError');
const InvalidSTPacketHashError = require('../../../../lib/errors/InvalidSTPacketHashError');
const InvalidTransactionTypeError = require('../../../../lib/errors/InvalidTransactionTypeError');
const ConsensusError = require('../../../../lib/errors/ConsensusError');

describe('verifySTPacketFactory', () => {
  let verifyContractMock;
  let verifyDocumentsMock;
  let transaction;
  let dataProviderMock;
  let verifySTPacket;
  let documents;
  let contract;
  let stPacket;
  let stateTransition;
  let userId;

  beforeEach(function beforeEach() {
    verifyContractMock = this.sinonSandbox.stub().resolves(new ValidationResult());
    verifyDocumentsMock = this.sinonSandbox.stub().resolves(new ValidationResult());

    dataProviderMock = createDataProviderMock(this.sinonSandbox);

    transaction = {
      confirmations: 6,
    };
    dataProviderMock.fetchTransaction.resolves(transaction);

    verifySTPacket = verifySTPacketFactory(
      verifyContractMock,
      verifyDocumentsMock,
      dataProviderMock,
    );

    ({ userId } = getDocumentsFixture);

    documents = getDocumentsFixture();
    contract = getContractFixture();

    stPacket = new STPacket(contract.getId());
    stPacket.setDocuments(documents);

    const payload = new Transaction.Payload.SubTxTransitionPayload()
      .setRegTxId(userId)
      .setHashPrevSubTx(userId)
      .setHashSTPacket(stPacket.hash())
      .setCreditFee(1001);

    stateTransition = new Transaction({
      type: Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION,
      extraPayload: payload.toString(),
    });

    dataProviderMock.fetchContract.resolves(contract);
  });

  it('should return invalid result if Transaction is not State Transition', async () => {
    const privateKey = new PrivateKey();
    const extraPayload = new Transaction.Payload.SubTxRegisterPayload()
      .setUserName('test')
      .setPubKeyIdFromPrivateKey(privateKey);

    stateTransition = new Transaction({
      type: Transaction.TYPES.TRANSACTION_SUBTX_REGISTER,
      extraPayload: extraPayload.toString(),
    });

    const result = await verifySTPacket(stPacket, stateTransition);

    expectValidationError(result, InvalidTransactionTypeError);

    const [error] = result.getErrors();

    expect(error.getTransaction()).to.equal(stateTransition);
  });

  it('should return invalid result if State Transition contains wrong ST Packet hash', async () => {
    stateTransition.extraPayload.hashSTPacket = 'ac5784e7dd8fc9f1b638a353fb10015d3841bb9076c20e2ebefc3e97599e92b5';

    const result = await verifySTPacket(stPacket, stateTransition);

    expectValidationError(result, InvalidSTPacketHashError);

    const [error] = result.getErrors();

    expect(error.getSTPacket()).to.equal(stPacket);
    expect(error.getStateTransition()).to.equal(stateTransition);
  });

  it('should return invalid result if user not found', async () => {
    dataProviderMock.fetchTransaction.resolves(null);

    const result = await verifySTPacket(stPacket, stateTransition);

    expectValidationError(result, UserNotFoundError);

    expect(dataProviderMock.fetchTransaction).to.have.been.calledOnceWith(userId);

    const [error] = result.getErrors();

    expect(error.getUserId()).to.equal(userId);
  });

  it('should return invalid result if user has less than 6 block confirmation', async () => {
    transaction.confirmations = 5;

    dataProviderMock.fetchTransaction.resolves(transaction);

    const result = await verifySTPacket(stPacket, stateTransition);

    expectValidationError(result, UnconfirmedUserError);

    expect(dataProviderMock.fetchTransaction).to.have.been.calledOnceWith(userId);

    const [error] = result.getErrors();

    expect(error.getRegistrationTransaction()).to.equal(transaction);
  });

  it('should return invalid result if Contract is not valid', async () => {
    stPacket.setDocuments([]);
    stPacket.setContract(contract);

    stateTransition.extraPayload.hashSTPacket = stPacket.hash();

    const expectedError = new ConsensusError('someError');
    verifyContractMock.resolves(
      new ValidationResult([expectedError]),
    );

    const result = await verifySTPacket(stPacket, stateTransition);

    expectValidationError(result);

    expect(dataProviderMock.fetchTransaction).to.have.been.calledOnceWith(userId);

    expect(verifyContractMock).to.have.been.calledOnceWith(stPacket);
    expect(verifyDocumentsMock).to.have.not.been.called();

    const [actualError] = result.getErrors();

    expect(actualError).to.equal(expectedError);
  });

  it('should return invalid result if Documents are not valid', async () => {
    const expectedError = new ConsensusError('someError');
    verifyDocumentsMock.resolves(
      new ValidationResult([expectedError]),
    );

    const result = await verifySTPacket(stPacket, stateTransition);

    expectValidationError(result);

    expect(dataProviderMock.fetchTransaction).to.have.been.calledOnceWith(userId);

    expect(verifyContractMock).to.have.not.been.called();
    expect(verifyDocumentsMock).to.have.been.calledOnceWith(stPacket, userId);

    const [actualError] = result.getErrors();

    expect(actualError).to.equal(expectedError);
  });

  it('should return valid result if ST Packet is valid', async () => {
    const result = await verifySTPacket(stPacket, stateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(dataProviderMock.fetchTransaction).to.have.been.calledOnceWith(userId);

    expect(verifyContractMock).to.have.not.been.called();
    expect(verifyDocumentsMock).to.have.been.calledOnceWith(stPacket, userId);
  });
});
