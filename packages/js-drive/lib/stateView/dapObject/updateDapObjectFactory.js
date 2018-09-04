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
    const dapObject = new DapObject(blockchainUserId, dapObjectData, reference);

    switch (dapObject.getAction()) {
      case DapObject.ACTION_CREATE:
      case DapObject.ACTION_UPDATE:
        await dapObjectRepository.store(dapObject);
        break;
      case DapObject.ACTION_DELETE:
        await dapObjectRepository.delete(dapObject);
        break;
      default:
    }
  }

  return updateDapObject;
}

module.exports = updateDapObjectFactory;
