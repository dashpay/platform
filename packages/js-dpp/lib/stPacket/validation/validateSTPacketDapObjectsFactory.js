const ValidationResult = require('../../validation/ValidationResult');

const DuplicatedDapObjectsError = require('../../errors/DuplicatedDapObjectsError');

/**
 * @param {validateDapObject} validateDapObject
 * @param {findDuplicatedDapObjects} findDuplicatedDapObjects
 * @return {validateSTPacketDapObjects}
 */
function validateSTPacketDapObjectsFactory(validateDapObject, findDuplicatedDapObjects) {
  /**
   * @typedef validateSTPacketDapObjects
   * @param {Object[]} rawDapObjects
   * @param {DapContract} dapContract
   * @return {ValidationResult}
   */
  function validateSTPacketDapObjects(rawDapObjects, dapContract) {
    const result = new ValidationResult();

    const duplicatedDapObjects = findDuplicatedDapObjects(rawDapObjects);
    if (duplicatedDapObjects.length) {
      result.addError(
        new DuplicatedDapObjectsError(duplicatedDapObjects),
      );
    }

    rawDapObjects.forEach((rawDapObject) => {
      result.merge(
        validateDapObject(rawDapObject, dapContract),
      );
    });

    return result;
  }

  return validateSTPacketDapObjects;
}

module.exports = validateSTPacketDapObjectsFactory;
