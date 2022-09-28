const AbstractStateError = require('../AbstractStateError');

class MaxIdentityPublicKeyLimitReachedError extends AbstractStateError {
  /**
   * @param {number} maxItems
   */
  constructor(maxItems) {
    super(`Identity cannot contain more than ${maxItems} public keys`);

    this.maxItems = maxItems;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   *
   * @return {number}
   */
  geMaxItems() {
    return this.maxItems;
  }
}

module.exports = MaxIdentityPublicKeyLimitReachedError;
