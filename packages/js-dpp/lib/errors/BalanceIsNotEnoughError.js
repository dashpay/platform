const ConsensusError = require('./ConsensusError');

class BalanceIsNotEnoughError extends ConsensusError {
  /**
   *
   * @param {number} balance
   */
  constructor(balance) {
    super('Balance is not enough');

    this.balance = balance;
  }

  /**
   * Get Identity balance
   * @return {number}
   */
  getBalance() {
    return this.balance;
  }
}

module.exports = BalanceIsNotEnoughError;
