const AbstractBasicError = require('../AbstractBasicError');

class MissingMasterPublicKeyError extends AbstractBasicError {
  constructor() {
    super('Identity doesn\'t contain any master key, thus can not be updated. Please add a master key');

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }
}

module.exports = MissingMasterPublicKeyError;
