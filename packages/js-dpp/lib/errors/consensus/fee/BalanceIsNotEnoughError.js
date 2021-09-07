const AbstractFeeError = require('./AbstractFeeError');

class BalanceIsNotEnoughError extends AbstractFeeError {
  /**
   * @param {number} balance
   * @param {number} fee
   */
  constructor(balance, fee) {
    super(`Current credits balance ${balance} is not enough to pay ${fee} fee`);

    this.balance = balance;
    this.fee = fee;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * @return {number}
   */
  getFee() {
    return this.fee;
  }

  /**
   * Get current balance
   *
   * @return {number}
   */
  getBalance() {
    return this.balance;
  }
}

module.exports = BalanceIsNotEnoughError;
