const DriveError = require('../../../errors/DriveError');

class NegativeBalanceError extends DriveError {
  /**
   * @param {Identity} identity
   */
  constructor(identity) {
    super(`Identity ${identity.getId()} has negative balance ${identity.getBalance()}`);
  }
}

module.exports = NegativeBalanceError;
