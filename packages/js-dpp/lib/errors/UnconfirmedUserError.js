const ConsensusError = require('./ConsensusError');

class UnconfirmedUserError extends ConsensusError {
  /**
   * @param {Object} registrationTransaction
   */
  constructor(registrationTransaction) {
    super('User has less than 6 confirmation');

    this.registrationTransaction = registrationTransaction;
  }

  /**
   * Get registration transaction
   *
   * @return {Object}
   */
  getRegistrationTransaction() {
    return this.registrationTransaction;
  }
}

module.exports = UnconfirmedUserError;
