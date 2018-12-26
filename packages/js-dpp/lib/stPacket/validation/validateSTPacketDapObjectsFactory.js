const ValidationResult = require('../../validation/ValidationResult');

const DuplicatedDapObjectsError = require('../../errors/DuplicatedDapObjectsError');
const InvalidDapContractError = require('../../errors/InvalidDapContractError');

/**
 * @param {validateDapObject} validateDapObject
 * @param {findDuplicatedDapObjects} findDuplicatedDapObjects
 * @return {validateSTPacketDapObjects}
 */
function validateSTPacketDapObjectsFactory(validateDapObject, findDuplicatedDapObjects) {
  /**
   * @typedef validateSTPacketDapObjects
   * @param {Object} rawSTPacket
   * @param {DapContract} dapContract
   * @return {ValidationResult}
   */
  function validateSTPacketDapObjects(rawSTPacket, dapContract) {
    const { objects: rawDapObjects } = rawSTPacket;

    const result = new ValidationResult();

    if (dapContract.getId() !== rawSTPacket.contractId) {
      result.addError(
        new InvalidDapContractError(dapContract, rawSTPacket),
      );
    }

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
