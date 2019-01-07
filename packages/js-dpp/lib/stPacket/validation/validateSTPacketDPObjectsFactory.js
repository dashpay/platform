const ValidationResult = require('../../validation/ValidationResult');

const DuplicatedDPObjectsError = require('../../errors/DuplicatedDPObjectsError');
const InvalidDPContractError = require('../../errors/InvalidDPContractError');

/**
 * @param {validateDPObject} validateDPObject
 * @param {findDuplicatedDPObjects} findDuplicatedDPObjects
 * @return {validateSTPacketDPObjects}
 */
function validateSTPacketDPObjectsFactory(validateDPObject, findDuplicatedDPObjects) {
  /**
   * @typedef validateSTPacketDPObjects
   * @param {Object} rawSTPacket
   * @param {DPContract} dpContract
   * @return {ValidationResult}
   */
  function validateSTPacketDPObjects(rawSTPacket, dpContract) {
    const { objects: rawDPObjects } = rawSTPacket;

    const result = new ValidationResult();

    if (dpContract.getId() !== rawSTPacket.contractId) {
      result.addError(
        new InvalidDPContractError(dpContract, rawSTPacket),
      );
    }

    const duplicatedDPObjects = findDuplicatedDPObjects(rawDPObjects);
    if (duplicatedDPObjects.length) {
      result.addError(
        new DuplicatedDPObjectsError(duplicatedDPObjects),
      );
    }

    rawDPObjects.forEach((rawDPObject) => {
      result.merge(
        validateDPObject(rawDPObject, dpContract),
      );
    });

    return result;
  }

  return validateSTPacketDPObjects;
}

module.exports = validateSTPacketDPObjectsFactory;
