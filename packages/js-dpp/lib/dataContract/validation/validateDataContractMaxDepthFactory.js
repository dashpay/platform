const lodashCloneDeep = require('lodash.clonedeep');
const ValidationResult = require('../../validation/ValidationResult');
const DataContractMaxDepthExceedError = require('../../errors/consensus/basic/dataContract/DataContractMaxDepthExceedError');
const InvalidJsonSchemaRefError = require('../../errors/consensus/basic/dataContract/InvalidJsonSchemaRefError');

/**
 * Check that JSON Schema max depth is less than max value
 * @private
 * @param {Object} json
 * @returns {number}
 */
function checkMaxDepth(json) {
  let depth = 1;

  const keys = Object.keys(json);

  for (let i = 0; i < keys.length; i++) {
    const key = keys[i];

    if (typeof json[key] === 'object') {
      const tmpDepth = checkMaxDepth(json[key]) + 1;
      depth = Math.max(depth, tmpDepth);
    }
  }

  if (depth > DataContractMaxDepthExceedError.MAX_DEPTH) {
    throw new DataContractMaxDepthExceedError();
  }

  return depth;
}

/**
 *
 * @param {$RefParser} refParser
 * @returns {validateDataContractMaxDepth}
 */
function validateDataContractMaxDepthFactory(refParser) {
  /**
   * Dereference JSON Schema $ref pointers and check max depth
   * @typedef validateDataContractMaxDepth
   * @param {RawDataContract} rawDataContract
   * @returns {ValidationResult}
   */
  async function validateDataContractMaxDepth(rawDataContract) {
    const result = new ValidationResult();
    let dereferencedDataContract;

    const clonedDataContract = lodashCloneDeep(rawDataContract);

    try {
      dereferencedDataContract = await refParser.dereference(clonedDataContract, {
        parse: {
          yaml: false,
          text: false,
          binary: false,
        },
        resolve: {
          external: false,
          http: false,
        },
        dereference: {
          circular: false,
        },
      });
    } catch (error) {
      const consensusError = new InvalidJsonSchemaRefError(error.message);

      consensusError.setRefError(error);

      result.addError(consensusError);

      return result;
    }

    try {
      checkMaxDepth(dereferencedDataContract);
    } catch (error) {
      if (error instanceof DataContractMaxDepthExceedError) {
        result.addError(error);

        return result;
      }

      throw error;
    }

    return result;
  }

  return validateDataContractMaxDepth;
}

module.exports = validateDataContractMaxDepthFactory;
