const AbstractSignatureError = require('./AbstractSignatureError');

class InvalidSignatureScriptError extends AbstractSignatureError {
  /**
   * @param {Buffer} signatureScript
   */
  constructor(signatureScript) {
    super('Invalid State Transition signature script');

    this.signatureScript = signatureScript;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   *
   * @returns {Buffer}
   */
  getSignatureScript() {
    return this.signatureScript;
  }
}

module.exports = InvalidSignatureScriptError;
