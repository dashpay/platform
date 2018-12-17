/**
 * @param {AbstractDataProvider} dataProvider
 * @return {fetchDapObjectsByObjects}
 */
function fetchDapObjectsByObjectsFactory(dataProvider) {
  /**
   * @typedef fetchDapObjectsByObjects
   * @param {string} dapContractId
   * @param {DapObject[]} dapObjects
   * @return {DapObject[]}
   */
  async function fetchDapObjectsByObjects(dapContractId, dapObjects) {
    // Group DAP Object IDs by types
    const dapObjectIdsByTypes = dapObjects.reduce((obj, dapObject) => {
      if (!obj[dapObject.getType()]) {
        // eslint-disable-next-line no-param-reassign
        obj[dapObject.getType()] = [];
      }

      obj[dapObject.getType()].push(dapObject.getId());

      return obj;
    }, {});

    // Convert object to array
    const dapObjectsArray = Object.entries(dapObjectIdsByTypes);

    // Fetch Dap Objects by IDs
    const fetchedDapObjectPromises = dapObjectsArray.map(([type, ids]) => {
      const options = {
        where: { id: { $in: ids } },
      };

      return dataProvider.fetchDapObjects(
        dapContractId,
        type,
        options,
      );
    });

    const fetchedDapObjectsByTypes = await Promise.all(fetchedDapObjectPromises);

    return fetchedDapObjectsByTypes.reduce((array, objects) => array.concat(...objects), []);
  }

  return fetchDapObjectsByObjects;
}

module.exports = fetchDapObjectsByObjectsFactory;
