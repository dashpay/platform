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
   * @returns {Promise<void>}
   */
  async function updateDapObject(dapId, blockchainUserId, reference, dapObjectData) {
    const dapObjectRepository = createDapObjectRepository(dapId);
    const markAsDeleted = false;
    const dapObject = new DapObject(blockchainUserId, markAsDeleted, dapObjectData, reference);

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
        dapObject.addRevision(previousDapObject);
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
