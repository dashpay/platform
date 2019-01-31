const DPObject = require('@dashevo/dpp/lib/object/DPObject');
const SVObject = require('./SVObject');

function updateSVObjectFactory(createSVObjectRepository) {
  /**
   * Generate DP Object State View
   *
   * @typedef {Promise} updateSVObject
   * @param {string} contractId
   * @param {string} userId
   * @param {Reference} reference
   * @param {DPObject} dpObject
   * @param {boolean} reverting
   * @returns {Promise<void>}
   */
  async function updateSVObject(contractId, userId, reference, dpObject, reverting) {
    const svObjectRepository = createSVObjectRepository(contractId, dpObject.getType());

    const svObject = new SVObject(userId, dpObject, reference);

    switch (dpObject.getAction()) {
      case DPObject.ACTIONS.CREATE: {
        await svObjectRepository.store(svObject);

        break;
      }

      case DPObject.ACTIONS.DELETE: {
        svObject.markAsDeleted();
      }
      // eslint-disable-next-line no-fallthrough
      case DPObject.ACTIONS.UPDATE: {
        const previousSVObject = await svObjectRepository.find(svObject.getDPObject().getId());

        if (!previousSVObject) {
          throw new Error('There is no object to update');
        }

        svObject.addRevision(previousSVObject);

        // NOTE: Since reverting is more complicated
        // `previousSVObject` is actually next one here
        // so we have to remove it's revision and the revision before that
        // to have a proper set of `previousRevisions`
        if (reverting) {
          svObject.removeAheadRevisions();
        }

        await svObjectRepository.store(svObject);

        break;
      }

      default: {
        throw new Error('Unsupported action');
      }
    }
  }

  return updateSVObject;
}

module.exports = updateSVObjectFactory;
