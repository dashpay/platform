const DapObject = require('../../dapObject/DapObject');

const ValidationResult = require('../../validation/ValidationResult');

const InvalidDapObjectActionError = require('../errors/InvalidDapObjectActionError');

const DapObjectAlreadyPresentError = require('../../errors/DapObjectAlreadyPresentError');
const DapObjectNotFoundError = require('../../errors/DapObjectNotFoundError');
const InvalidDapObjectRevisionError = require('../../errors/InvalidDapObjectRevisionError');
const InvalidDapObjectScopeError = require('../../errors/InvalidDapObjectScopeError');

const hash = require('../../util/hash');

/**
 * @param {fetchDapObjectsByObjects} fetchDapObjectsByObjects
 * @return {verifyDapObjects}
 */
function verifyDapObjectsFactory(fetchDapObjectsByObjects) {
  /**
   * @typedef verifyDapObjects
   * @param {STPacket} stPacket
   * @param {string} userId
   * @return {ValidationResult}
   */
  async function verifyDapObjects(stPacket, userId) {
    const result = new ValidationResult();

    const fetchedDapObjects = await fetchDapObjectsByObjects(
      stPacket.getDapContractId(),
      stPacket.getDapObjects(),
    );

    stPacket.getDapObjects().forEach((dapObject) => {
      const fetchedDapObject = fetchedDapObjects.find(o => dapObject.getId() === o.getId());

      const stPacketScope = hash(stPacket.getDapContractId() + userId);
      if (dapObject.scope !== stPacketScope) {
        result.addError(
          new InvalidDapObjectScopeError(dapObject),
        );
      }

      switch (dapObject.getAction()) {
        case DapObject.ACTIONS.CREATE:
          if (fetchedDapObject) {
            result.addError(
              new DapObjectAlreadyPresentError(dapObject, fetchedDapObject),
            );
          }
          break;
        case DapObject.ACTIONS.UPDATE:
        case DapObject.ACTIONS.DELETE:
          if (!fetchedDapObject) {
            result.addError(
              new DapObjectNotFoundError(dapObject),
            );

            break;
          }

          if (dapObject.getRevision() !== fetchedDapObject.getRevision() + 1) {
            result.addError(
              new InvalidDapObjectRevisionError(dapObject, fetchedDapObject),
            );
          }

          break;
        default:
          throw new InvalidDapObjectActionError(dapObject);
      }
    });

    return result;
  }

  return verifyDapObjects;
}

module.exports = verifyDapObjectsFactory;
