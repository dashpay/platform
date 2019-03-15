const DPObject = require('../../object/DPObject');

/**
 * @param {DPObject} originalObject
 * @param {DPObject} objectToCheck
 * @param {Object} typeIndices
 *
 * @return {boolean}
 */
function isDuplicateByIndices(originalObject, objectToCheck, typeIndices) {
  return typeIndices
    // For every index definition check if hashes match
    // accumulating overall boolean result
    .reduce((accumulator, definition) => {
      const [originalHash, hashToCheck] = definition.properties
        .reduce(([originalAcc, toCheckAcc], property) => {
          const propertyName = Object.keys(property)[0];
          return [
            `${originalAcc}:${originalObject.get(propertyName)}`,
            `${toCheckAcc}:${objectToCheck.get(propertyName)}`,
          ];
        }, ['', '']);

      return accumulator || (originalHash === hashToCheck);
    }, false);
}

/**
 * Find duplicate objects by unique indices
 *
 * @typedef findDuplicateDPObjectsByIndices
 *
 * @param {Object[]} rawDPObjects
 * @param {DPContract} dpContract
 *
 * @return {Object[]}
 */
function findDuplicateDPObjectsByIndices(rawDPObjects, dpContract) {
  // Convert raw objects to DPObject instances
  const dpObjects = rawDPObjects.map(o => new DPObject(o));

  const groupsObject = dpObjects
    // Group objects by it's type, enrich them by type's unique indices
    .reduce((groups, dpObject) => {
      const type = dpObject.getType();
      const typeIndices = (dpContract.getDPObjectSchema(type).indices || []);

      // Init empty group
      if (!groups[type]) {
        // eslint-disable-next-line no-param-reassign
        groups[type] = {
          items: [],
          // init group with only it's unique indices
          indices: typeIndices.filter(index => index.unique),
        };
      }

      groups[type].items.push(dpObject);

      return groups;
    }, {});

  const duplicateArrays = Object.values(groupsObject)
    // Filter out groups without unique indices
    .filter(group => group.indices.length > 0)
    // Filter out groups with only one object
    .filter(group => group.items.length > 1)
    .map(group => group.items
      // Flat map found duplicates in a group
      .reduce((foundGroupObjects, dpObject) => {
        // For every object in a group make duplicate search
        const objects = group.items
          // Exclude current object from search
          .filter(o => o.getId() !== dpObject.getId())
          .reduce((foundObjects, objectToCheck) => {
            if (isDuplicateByIndices(dpObject, objectToCheck, group.indices)) {
              foundObjects.push(objectToCheck);
            }
            return foundObjects;
          }, []);

        return foundGroupObjects.concat(objects);
      }, []));

  // Flat map the results and return raw objects
  return duplicateArrays
    .reduce((accumulator, items) => accumulator.concat(items), [])
    .map(o => o.toJSON());
}

module.exports = findDuplicateDPObjectsByIndices;
