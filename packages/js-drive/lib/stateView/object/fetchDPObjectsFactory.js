/**
 * @param {createSVObjectMongoDbRepository} createSVObjectRepository
 * @returns {fetchDPObjects}
 */
function fetchDPObjectsFactory(createSVObjectRepository) {
  /**
   * Fetch original DP Objects by DP Contract ID and type
   *
   * @typedef {Promise} fetchDPObjects
   * @param {string} contractId
   * @param {string} type
   * @param {Object} [options] options
   * @returns {DPObject[]}
   */
  async function fetchDPObjects(contractId, type, options) {
    const svObjectRepository = createSVObjectRepository(contractId, type);
    const svObjects = await svObjectRepository.fetch(options);
    return svObjects.map(svObject => svObject.getDPObject());
  }

  return fetchDPObjects;
}

module.exports = fetchDPObjectsFactory;
