const { Transaction } = require('@dashevo/dashcore-lib');

const ValidationResult = require('../../validation/ValidationResult');

const UserNotFoundError = require('../../errors/UserNotFoundError');
const UnconfirmedUserError = require('../../errors/UnconfirmedUserError');
const InvalidRegistrationTransactionTypeError = require('../../errors/InvalidRegistrationTransactionTypeError');

const MIN_CONFIRMATIONS = 6;

function validateBlockchainUserFactory(dataProvider) {
  /**
   * @typedef validateBlockchainUser
   * @param {string} userId
   * @return {Promise<ValidationResult>}
   */
  async function validateBlockchainUser(userId) {
    const result = new ValidationResult();

    const rawRegistrationTransaction = await dataProvider.fetchTransaction(userId);

    if (!rawRegistrationTransaction) {
      result.addError(
        new UserNotFoundError(userId),
      );

      return result;
    }

    if (rawRegistrationTransaction.confirmations < MIN_CONFIRMATIONS) {
      result.addError(
        new UnconfirmedUserError(rawRegistrationTransaction),
      );
    }

    if (rawRegistrationTransaction.type !== Transaction.TYPES.TRANSACTION_SUBTX_REGISTER) {
      result.addError(
        new InvalidRegistrationTransactionTypeError(rawRegistrationTransaction),
      );
    }

    return result;
  }

  return validateBlockchainUser;
}

module.exports = validateBlockchainUserFactory;
