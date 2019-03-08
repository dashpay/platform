const { Transaction, PrivateKey } = require('@dashevo/dashcore-lib');

const STPacket = require('../../../../lib/stPacket/STPacket');

const getDPObjectsFixture = require('../../../../lib/test/fixtures/getDPObjectsFixture');
const getDPContractFixture = require('../../../../lib/test/fixtures/getDPContractFixture');

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
  let verifyDPContractMock;
  let verifyDPObjectsMock;
  let transaction;
  let dataProviderMock;
  let verifySTPacket;
  let dpObjects;
  let dpContract;
  let stPacket;
  let stateTransition;
  let userId;

  beforeEach(function beforeEach() {
    verifyDPContractMock = this.sinonSandbox.stub().resolves(new ValidationResult());
    verifyDPObjectsMock = this.sinonSandbox.stub().resolves(new ValidationResult());

    dataProviderMock = createDataProviderMock(this.sinonSandbox);

    transaction = {
      confirmations: 6,
    };
    dataProviderMock.fetchTransaction.resolves(transaction);

    verifySTPacket = verifySTPacketFactory(
      verifyDPContractMock,
      verifyDPObjectsMock,
      dataProviderMock,
    );

    ({ userId } = getDPObjectsFixture);

    dpObjects = getDPObjectsFixture();
    dpContract = getDPContractFixture();

    stPacket = new STPacket(dpContract.getId());
    stPacket.setDPObjects(dpObjects);

    stateTransition = new Transaction({
      type: Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION,
      extraPayload: {
        version: 1,
        hashSTPacket: stPacket.hash(),
        regTxId: userId,
        creditFee: 1001,
        hashPrevSubTx: userId,
      },
    });
  });

  it('should return invalid result if Transaction is not State Transition', async () => {
    const privateKey = new PrivateKey();
    const extraPayload = new Transaction.Payload.SubTxRegisterPayload()
      .setUserName('test')
      .setPubKeyIdFromPrivateKey(privateKey);

    stateTransition = new Transaction({
      type: Transaction.TYPES.TRANSACTION_SUBTX_REGISTER,
      extraPayload,
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

  it('should return invalid result if DP Contract is not valid', async () => {
    stPacket.setDPObjects([]);
    stPacket.setDPContract(dpContract);

    stateTransition.extraPayload.hashSTPacket = stPacket.hash();

    const expectedError = new ConsensusError('someError');
    verifyDPContractMock.resolves(
      new ValidationResult([expectedError]),
    );

    const result = await verifySTPacket(stPacket, stateTransition);

    expectValidationError(result);

    expect(dataProviderMock.fetchTransaction).to.have.been.calledOnceWith(userId);

    expect(verifyDPContractMock).to.have.been.calledOnceWith(stPacket);
    expect(verifyDPObjectsMock).to.have.not.been.called();

    const [actualError] = result.getErrors();

    expect(actualError).to.equal(expectedError);
  });

  it('should return invalid result if DPObjects are not valid', async () => {
    const expectedError = new ConsensusError('someError');
    verifyDPObjectsMock.resolves(
      new ValidationResult([expectedError]),
    );

    const result = await verifySTPacket(stPacket, stateTransition);

    expectValidationError(result);

    expect(dataProviderMock.fetchTransaction).to.have.been.calledOnceWith(userId);

    expect(verifyDPContractMock).to.have.not.been.called();
    expect(verifyDPObjectsMock).to.have.been.calledOnceWith(stPacket, userId);

    const [actualError] = result.getErrors();

    expect(actualError).to.equal(expectedError);
  });

  it('should return valid result if ST Packet is valid', async () => {
    const result = await verifySTPacket(stPacket, stateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(dataProviderMock.fetchTransaction).to.have.been.calledOnceWith(userId);

    expect(verifyDPContractMock).to.have.not.been.called();
    expect(verifyDPObjectsMock).to.have.been.calledOnceWith(stPacket, userId);
  });
});
