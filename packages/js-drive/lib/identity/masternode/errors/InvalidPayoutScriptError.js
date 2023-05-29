const DriveError = require('../../../errors/DriveError');

class InvalidPayoutScriptError extends DriveError {
  /**
   * @param {Buffer} payoutScript
   */
  constructor(payoutScript) {
    super('Invalid payout script');

    this.payoutScript = payoutScript;
  }

  /**
   *
   * @return {Buffer}
   */
  getPayoutScript() {
    return this.payoutScript;
  }
}

module.exports = InvalidPayoutScriptError;
