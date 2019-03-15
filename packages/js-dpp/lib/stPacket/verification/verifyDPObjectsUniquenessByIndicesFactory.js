const bs58 = require('bs58');

const ValidationResult = require('../../validation/ValidationResult');
const DuplicateDPObjectError = require('../../errors/DuplicateDPObjectError');

/**
 * @param {fetchDPObjectsByObjects} fetchDPObjectsByObjects
 * @param {DataProvider} dataProvider
 * @return {verifyDPObjectsUniquenessByIndices}
 */
function verifyDPObjectsUniquenessByIndicesFactory(fetchDPObjectsByObjects, dataProvider) {
  /**
   * @typedef verifyDPObjectsUniquenessByIndices
   * @param {STPacket} stPacket
   * @param {string} userId
   * @param {DPContract} dpContract
   * @return {ValidationResult}
   */
  async function verifyDPObjectsUniquenessByIndices(stPacket, userId, dpContract) {
    const result = new ValidationResult();

    // 1. Prepare fetchDPObjects queries from indexed properties
    const dpObjectIndexQueries = stPacket.getDPObjects()
      .reduce((queries, dpObject) => {
        const dpObjectSchema = dpContract.getDPObjectSchema(dpObject.getType());

        if (!dpObjectSchema.indices) {
          return queries;
        }

        dpObjectSchema.indices
          .filter(index => index.unique)
          .forEach((indexDefinition) => {
            const where = indexDefinition.properties
              .reduce((obj, property) => {
                const propertyName = Object.keys(property)[0];
                // eslint-disable-next-line no-param-reassign
                obj[`dpObject.${propertyName}`] = dpObject.get(propertyName);

                return obj;
              }, {});

            // Exclude origin DP Object
            const idBuffer = Buffer.from(dpObject.getId(), 'hex');
            // eslint-disable-next-line no-underscore-dangle
            where._id = { $ne: bs58.encode(idBuffer) };

            queries.push({
              type: dpObject.getType(),
              indexDefinition,
              originDPObject: dpObject,
              where,
            });
          });

        return queries;
      }, []);

    // 2. Fetch DP Object by indexed properties
    const fetchDPObjectPromises = dpObjectIndexQueries
      .map(({
        type,
        where,
        indexDefinition,
        originDPObject,
      }) => (
        dataProvider.fetchDPObjects(
          stPacket.getDPContractId(),
          type,
          { where },
        )
          .then(dpObjects => Object.assign(dpObjects, {
            indexDefinition,
            originDPObject,
          }))
      ));

    const fetchedDPObjectsByIndices = await Promise.all(fetchDPObjectPromises);

    // 3. Create errors if duplicates found
    fetchedDPObjectsByIndices
      .filter(dpObjects => dpObjects.length !== 0)
      .forEach((dpObjects) => {
        result.addError(
          new DuplicateDPObjectError(
            dpObjects.originDPObject,
            dpObjects.indexDefinition,
          ),
        );
      });

    return result;
  }

  return verifyDPObjectsUniquenessByIndices;
}

module.exports = verifyDPObjectsUniquenessByIndicesFactory;
