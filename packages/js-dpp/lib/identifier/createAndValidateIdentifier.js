const Identifier = require('./Identifier');
const InvalidIdentifierError = require('../errors/consensus/basic/InvalidIdentifierError');
const IdentifierError = require('./errors/IdentifierError');

/**
 * @param {string} name
 * @param {Buffer} buffer
 * @param {ValidationResult} result
 * @return {Identifier}
 */
function createAndValidateIdentifier(name, buffer, result) {
  try {
    return new Identifier(buffer);
  } catch (e) {
    if (e instanceof IdentifierError) {
      const consensusError = new InvalidIdentifierError(name, e.message);

      consensusError.setIdentifierError(e);

      result.addError(consensusError);

      return undefined;
    }

    throw e;
  }
}

module.exports = createAndValidateIdentifier;
