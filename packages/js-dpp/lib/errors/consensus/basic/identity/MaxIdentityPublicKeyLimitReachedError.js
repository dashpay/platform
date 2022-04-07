const AbstractBasicError = require('../AbstractBasicError');

class MaxIdentityPublicKeyLimitReachedError extends AbstractBasicError {
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
