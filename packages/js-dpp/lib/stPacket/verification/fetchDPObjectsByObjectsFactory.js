/**
 * @param {DataProvider} dataProvider
 * @return {fetchDPObjectsByObjects}
 */
function fetchDPObjectsByObjectsFactory(dataProvider) {
  /**
   * @typedef fetchDPObjectsByObjects
   * @param {string} dpContractId
   * @param {DPObject[]} dpObjects
   * @return {DPObject[]}
   */
  async function fetchDPObjectsByObjects(dpContractId, dpObjects) {
    // Group DP Object IDs by types
    const dpObjectIdsByTypes = dpObjects.reduce((obj, dpObject) => {
      if (!obj[dpObject.getType()]) {
        // eslint-disable-next-line no-param-reassign
        obj[dpObject.getType()] = [];
      }

      obj[dpObject.getType()].push(dpObject.getId());

      return obj;
    }, {});

    // Convert object to array
    const dpObjectsArray = Object.entries(dpObjectIdsByTypes);

    // Fetch DPObjects by IDs
    const fetchedDPObjectPromises = dpObjectsArray.map(([type, ids]) => {
      const options = {
        where: { id: { $in: ids } },
      };

      return dataProvider.fetchDPObjects(
        dpContractId,
        type,
        options,
      );
    });

    const fetchedDPObjectsByTypes = await Promise.all(fetchedDPObjectPromises);

    return fetchedDPObjectsByTypes.reduce((array, objects) => array.concat(...objects), []);
  }

  return fetchDPObjectsByObjects;
}

module.exports = fetchDPObjectsByObjectsFactory;
