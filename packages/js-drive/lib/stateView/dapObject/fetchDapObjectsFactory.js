/**
 * @param {createDapObjectMongoDbRepository} createDapObjectRepository
 * @returns {fetchDapObjects}
 */
function fetchDapObjectsFactory(createDapObjectRepository) {
  /**
   * Fetch original Dap Objects by DAP id and type
   *
   * @typedef {Promise} fetchDapObjects
   * @param {string} dapId
   * @param {string} type
   * @param {object} [options] options
   * @returns {Promise<DapObject[]>}
   */
  async function fetchDapObjects(dapId, type, options) {
    const dapObjectRepository = createDapObjectRepository(dapId);
    const dapObjects = await dapObjectRepository.fetch(type, options);
    return dapObjects.map(dapObject => dapObject.getOriginalData());
  }

  return fetchDapObjects;
}

module.exports = fetchDapObjectsFactory;
