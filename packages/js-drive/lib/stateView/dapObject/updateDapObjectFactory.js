const DapObject = require('./DapObject');

function updateDapObjectFactory(createDapObjectRepository) {
  /**
   * Generate DAP object State View
   *
   * @typedef {Promise} updateDapObject
   * @param {string} dapId
   * @param {string} blockchainUserId
   * @param {Reference} reference
   * @param {object} dapObjectData
   * @param {boolean} reverting
   * @returns {Promise<void>}
   */
  async function updateDapObject(dapId, blockchainUserId, reference, dapObjectData, reverting) {
    const dapObjectRepository = createDapObjectRepository(dapId);
    const dapObject = new DapObject(blockchainUserId, dapObjectData, reference, false);

    switch (dapObject.getAction()) {
      case DapObject.ACTION_CREATE: {
        await dapObjectRepository.store(dapObject);
        break;
      }
      case DapObject.ACTION_UPDATE: {
        const previousDapObject = await dapObjectRepository.find(dapObject.getId());
        if (!previousDapObject) {
          return;
        }
        dapObject.addRevision(previousDapObject, reverting);

        // NOTE: Since reverting is more complicated
        // `previousDapObject` is actually next one here
        // so we have to remove it's revision and the revision before that
        // to have a proper set of `previousRevisions`
        if (reverting) {
          dapObject.removeAheadRevisions();
        }

        await dapObjectRepository.store(dapObject);
        break;
      }
      case DapObject.ACTION_DELETE: {
        dapObject.markAsDeleted();
        await dapObjectRepository.store(dapObject);
        break;
      }
      default:
    }
  }

  return updateDapObject;
}

module.exports = updateDapObjectFactory;
