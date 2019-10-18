const { Transaction } = require('@dashevo/dashcore-lib');

const validateBlockchainUserFactory = require('../../../../lib/stateTransition/validation/validateBlockchainUserFactory');

const createDataProviderMock = require('../../../../lib/test/mocks/createDataProviderMock');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

const UserNotFoundError = require('../../../../lib/errors/UserNotFoundError');
const UnconfirmedUserError = require('../../../../lib/errors/UnconfirmedUserError');
const InvalidRegistrationTransactionTypeError = require('../../../../lib/errors/InvalidRegistrationTransactionTypeError');

describe('validateBlockchainUserFactory', () => {
  let validateBlockchainUser;
  let dataProviderMock;
  let userId;
  let rawTransaction;

  beforeEach(function beforeEach() {
    dataProviderMock = createDataProviderMock(this.sinonSandbox);

    validateBlockchainUser = validateBlockchainUserFactory(
      dataProviderMock,
    );

    userId = '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288';

    rawTransaction = {
      confirmations: 6,
      type: Transaction.TYPES.TRANSACTION_SUBTX_REGISTER,
    };
  });

  it('should return invalid result if transition is not found', async () => {
    const result = await validateBlockchainUser(userId);

    expectValidationError(result, UserNotFoundError);

    const [error] = result.getErrors();

    expect(error.getUserId()).to.equal(userId);
  });

  it('should return invalid result if transition is not confirmed', async () => {
    rawTransaction.confirmations = 5;

    dataProviderMock.fetchTransaction.resolves(rawTransaction);

    const result = await validateBlockchainUser(userId);

    expectValidationError(result, UnconfirmedUserError);

    const [error] = result.getErrors();

    expect(error.getRegistrationTransaction()).to.equal(rawTransaction);

    expect(dataProviderMock.fetchTransaction).to.be.calledOnceWith(userId);
  });

  it('should return invalid result if transition is not registration transaction', async () => {
    rawTransaction.type = Transaction.TYPES.TRANSACTION_NORMAL;

    dataProviderMock.fetchTransaction.resolves(rawTransaction);

    const result = await validateBlockchainUser(userId);

    expectValidationError(result, InvalidRegistrationTransactionTypeError);

    const [error] = result.getErrors();

    expect(error.getRawTransaction()).to.equal(rawTransaction);

    expect(dataProviderMock.fetchTransaction).to.be.calledOnceWith(userId);
  });

  it('should return valid result', async () => {
    dataProviderMock.fetchTransaction.resolves(rawTransaction);

    const result = await validateBlockchainUser(userId);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
