const { Transaction } = require('@dashevo/dashcore-lib');

const AbstractDataProvider = require('../../../../lib/dataProvider/AbstractDataProvider');

const STPacket = require('../../../../lib/stPacket/STPacket');

const getDapObjectsFixture = require('../../../../lib/test/fixtures/getDapObjectsFixture');
const getDapContractFixture = require('../../../../lib/test/fixtures/getDapContractFixture');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const verifySTPacketFactory = require('../../../../lib/stPacket/verification/verifySTPacketFactory');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

const UserNotFoundError = require('../../../../lib/errors/UserNotFoundError');
const UnconfirmedUserError = require('../../../../lib/errors/UnconfirmedUserError');
const InvalidSTPacketHashError = require('../../../../lib/errors/InvalidSTPacketHashError');
const ConsensusError = require('../../../../lib/errors/ConsensusError');

describe('verifySTPacketFactory', () => {
  let verifyDapContractMock;
  let verifyDapObjectsMock;
  let transaction;
  let fetchTransactionMock;
  let verifySTPacket;
  let dapObjects;
  let dapContract;
  let stPacket;
  let stateTransition;
  let userId;

  beforeEach(function beforeEach() {
    verifyDapContractMock = this.sinonSandbox.stub().resolves(new ValidationResult());
    verifyDapObjectsMock = this.sinonSandbox.stub().resolves(new ValidationResult());

    const dataProviderMock = this.sinonSandbox.createStubInstance(AbstractDataProvider, {
      fetchTransaction: this.sinonSandbox.stub(),
    });

    fetchTransactionMock = dataProviderMock.fetchTransaction;

    transaction = {
      confirmations: 6,
    };
    fetchTransactionMock.resolves(transaction);

    verifySTPacket = verifySTPacketFactory(
      verifyDapContractMock,
      verifyDapObjectsMock,
      dataProviderMock,
    );

    ({ userId } = getDapObjectsFixture);

    dapObjects = getDapObjectsFixture();
    dapContract = getDapContractFixture();

    stPacket = new STPacket(dapContract.getId());
    stPacket.setDapObjects(dapObjects);

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

  it('should return invalid result if State Transition contains wrong ST Packet hash', async () => {
    stateTransition.extraPayload.hashSTPacket = 'ac5784e7dd8fc9f1b638a353fb10015d3841bb9076c20e2ebefc3e97599e92b5';

    const result = await verifySTPacket(stPacket, stateTransition);

    expectValidationError(result, InvalidSTPacketHashError);

    const [error] = result.getErrors();

    expect(error.getSTPacket()).to.be.equal(stPacket);
    expect(error.getStateTransition()).to.be.equal(stateTransition);
  });

  it('should return invalid result if user not found', async () => {
    fetchTransactionMock.resolves(null);

    const result = await verifySTPacket(stPacket, stateTransition);

    expectValidationError(result, UserNotFoundError);

    expect(fetchTransactionMock).to.be.calledOnceWith(userId);

    const [error] = result.getErrors();

    expect(error.getUserId()).to.be.equal(userId);
  });

  it('should return invalid result if user has less than 6 block confirmation', async () => {
    transaction.confirmations = 5;

    fetchTransactionMock.resolves(transaction);

    const result = await verifySTPacket(stPacket, stateTransition);

    expectValidationError(result, UnconfirmedUserError);

    expect(fetchTransactionMock).to.be.calledOnceWith(userId);

    const [error] = result.getErrors();

    expect(error.getRegistrationTransaction()).to.be.equal(transaction);
  });

  it('should return invalid result if Dap Contract is not valid', async () => {
    stPacket.setDapObjects([]);
    stPacket.setDapContract(dapContract);

    stateTransition.extraPayload.hashSTPacket = stPacket.hash();

    const expectedError = new ConsensusError('someError');
    verifyDapContractMock.resolves(
      new ValidationResult([expectedError]),
    );

    const result = await verifySTPacket(stPacket, stateTransition);

    expectValidationError(result);

    expect(fetchTransactionMock).to.be.calledOnceWith(userId);

    expect(verifyDapContractMock).to.be.calledOnceWith(stPacket);
    expect(verifyDapObjectsMock).to.be.not.called();

    const [actualError] = result.getErrors();

    expect(actualError).to.be.equal(expectedError);
  });

  it('should return invalid result if Dap Objects are not valid', async () => {
    const expectedError = new ConsensusError('someError');
    verifyDapObjectsMock.resolves(
      new ValidationResult([expectedError]),
    );

    const result = await verifySTPacket(stPacket, stateTransition);

    expectValidationError(result);

    expect(fetchTransactionMock).to.be.calledOnceWith(userId);

    expect(verifyDapContractMock).to.be.not.called();
    expect(verifyDapObjectsMock).to.be.calledOnceWith(stPacket, userId);

    const [actualError] = result.getErrors();

    expect(actualError).to.be.equal(expectedError);
  });

  it('should return valid result if ST Packet is valid', async () => {
    const result = await verifySTPacket(stPacket, stateTransition);

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(fetchTransactionMock).to.be.calledOnceWith(userId);

    expect(verifyDapContractMock).to.be.not.called();
    expect(verifyDapObjectsMock).to.be.calledOnceWith(stPacket, userId);
  });
});
