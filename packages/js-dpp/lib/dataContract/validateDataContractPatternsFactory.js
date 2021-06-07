const traverse = require('json-schema-traverse');
const ValidationResult = require('../validation/ValidationResult');
const IncompatibleRe2PatternError = require('../document/errors/IncompatibleRe2PatternError');

/**
 *
 * @param {RE2} RE2
 * @return validateDataContractPatterns
 */
function validateDataContractPatternsFactory(
  RE2,
) {
  /**
   * @typedef validateDataContractPatterns
   * @param {RawDataContract} rawDataContract
   * @returns {ValidationResult}
   */
  function validateDataContractPatterns(rawDataContract) {
    const result = new ValidationResult();

    traverse(rawDataContract, {
      allKeys: true,
      cb: (item, path) => {
        Object.entries(item).forEach(([key, value]) => {
          if (key === 'pattern') {
            try {
              // eslint-disable-next-line no-new
              new RE2(value, 'u');
            } catch (e) {
              result.addError(new IncompatibleRe2PatternError(value, path, e.message));
            }
          }
        });
      },
    });

    return result;
  }

  return validateDataContractPatterns;
}

module.exports = validateDataContractPatternsFactory;
