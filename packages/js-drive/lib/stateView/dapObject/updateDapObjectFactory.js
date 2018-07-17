const DapObject = require('./DapObject');

function updateDapObjectFactory(createDapObjectRepository) {
  /**
   * Generate DAP object State View
   *
   * @typedef {Promise} updateDapObject
   * @param {string} dapId
   * @param {Reference} reference
   * @param {object} dapObjectData
   * @returns {Promise<void>}
   */
  async function updateDapObject(dapId, reference, dapObjectData) {
    const dapObjectRepository = createDapObjectRepository(dapId);
    const dapObject = new DapObject(dapObjectData, reference);
    if (dapObject.isNew()) {
      await dapObjectRepository.store(dapObject);
    }
  }

  return updateDapObject;
}

module.exports = updateDapObjectFactory;
