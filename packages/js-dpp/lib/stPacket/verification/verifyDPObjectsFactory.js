const DPObject = require('../../object/DPObject');

const ValidationResult = require('../../validation/ValidationResult');

const InvalidDPObjectActionError = require('../errors/InvalidDPObjectActionError');

const DPObjectAlreadyPresentError = require('../../errors/DPObjectAlreadyPresentError');
const DPObjectNotFoundError = require('../../errors/DPObjectNotFoundError');
const InvalidDPObjectRevisionError = require('../../errors/InvalidDPObjectRevisionError');
const InvalidDPObjectScopeError = require('../../errors/InvalidDPObjectScopeError');

const hash = require('../../util/hash');

/**
 * @param {fetchDPObjectsByObjects} fetchDPObjectsByObjects
 * @param {verifyDPObjectsUniquenessByIndices} verifyDPObjectsUniquenessByIndices
 * @return {verifyDPObjects}
 */
function verifyDPObjectsFactory(fetchDPObjectsByObjects, verifyDPObjectsUniquenessByIndices) {
  /**
   * @typedef verifyDPObjects
   * @param {STPacket} stPacket
   * @param {string} userId
   * @param {DPContract} dpContract
   * @return {ValidationResult}
   */
  async function verifyDPObjects(stPacket, userId, dpContract) {
    const result = new ValidationResult();

    const fetchedDPObjects = await fetchDPObjectsByObjects(
      stPacket.getDPContractId(),
      stPacket.getDPObjects(),
    );

    stPacket.getDPObjects()
      .forEach((dpObject) => {
        const fetchedDPObject = fetchedDPObjects.find(o => dpObject.getId() === o.getId());

        const stPacketScope = hash(stPacket.getDPContractId() + userId);
        if (dpObject.scope !== stPacketScope) {
          result.addError(
            new InvalidDPObjectScopeError(dpObject),
          );
        }

        switch (dpObject.getAction()) {
          case DPObject.ACTIONS.CREATE:
            if (fetchedDPObject) {
              result.addError(
                new DPObjectAlreadyPresentError(dpObject, fetchedDPObject),
              );
            }
            break;
          case DPObject.ACTIONS.UPDATE:
          case DPObject.ACTIONS.DELETE:
            if (!fetchedDPObject) {
              result.addError(
                new DPObjectNotFoundError(dpObject),
              );

              break;
            }

            if (dpObject.getRevision() !== fetchedDPObject.getRevision() + 1) {
              result.addError(
                new InvalidDPObjectRevisionError(dpObject, fetchedDPObject),
              );
            }

            break;
          default:
            throw new InvalidDPObjectActionError(dpObject);
        }
      });

    result.merge(
      await verifyDPObjectsUniquenessByIndices(stPacket, userId, dpContract),
    );

    return result;
  }

  return verifyDPObjects;
}

module.exports = verifyDPObjectsFactory;
