/**
 * @param {createDapObjectMongoDbRepository} createDapObjectRepository
 * @returns {fetchDapObjects}
 */
function fetchDapObjectsFactory(createDapObjectRepository) {
  /**
   * Fetch Dap Objects by DAP id and type
   *
   * @typedef {Promise} fetchDapObjects
   * @param {string} dapId
   * @param {string} type
   * @param {object} [options] options
   * @returns {Promise<DapObject[]>}
   */
  async function fetchDapObjects(dapId, type, options) {
    const dapObjectRepository = createDapObjectRepository(dapId);
    return dapObjectRepository.fetch(type, options);
  }

  return fetchDapObjects;
}

module.exports = fetchDapObjectsFactory;
