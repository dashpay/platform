const AbstractStateError = require('../AbstractStateError');

class IdentityPublicKeyDisabledAtWindowViolationError extends AbstractStateError {
  /**
   * @param {Date} disabledAt
   * @param {Date} timeWindowStart
   * @param {Date} timeWindowEnd
   */
  constructor(disabledAt, timeWindowStart, timeWindowEnd) {
    super(`Identity public keys disabled time (${disabledAt}) is out of block time window from ${timeWindowStart} and ${timeWindowEnd}`);

    this.disabledAt = disabledAt;
    this.timeWindowStart = timeWindowStart;
    this.timeWindowEnd = timeWindowEnd;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get disabledAt
   *
   * @return {Date}
   */
  getDisabledAt() {
    return this.disabledAt;
  }

  /**
   * @returns {Date}
   */
  getTimeWindowStart() {
    return this.timeWindowStart;
  }

  /**
   * @returns {Date}
   */
  getTimeWindowEnd() {
    return this.timeWindowEnd;
  }
}

module.exports = IdentityPublicKeyDisabledAtWindowViolationError;
