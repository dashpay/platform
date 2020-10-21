const Identifier = require('./Identifier');
const InvalidIdentifierError = require('../errors/InvalidIdentifierError');
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
      result.addError(
        new InvalidIdentifierError('$dataContractId', e),
      );

      return undefined;
    }

    throw e;
  }
}

module.exports = createAndValidateIdentifier;
