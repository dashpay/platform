const AbstractSignatureError = require('./AbstractSignatureError');

class MasterPublicKeySecurityLevelIsNotAllowedError extends AbstractSignatureError {
  constructor() {
    super('Master public key is not allowed to sign this transition type');

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }
}

module.exports = MasterPublicKeySecurityLevelIsNotAllowedError;
