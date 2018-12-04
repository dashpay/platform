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
   * @param {ValidationResult} result
   */
  function validateSTPacketDapObjects(rawDapObjects, dapContract, result) {
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
  }

  return validateSTPacketDapObjects;
}

module.exports = validateSTPacketDapObjectsFactory;
