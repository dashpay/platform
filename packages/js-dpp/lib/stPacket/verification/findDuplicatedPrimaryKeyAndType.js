const findDuplicates = require('../../util/findDuplicates');

/**
 * Find duplicates
 *
 * @param {DapObject[]} dapObjects
 * @return {DapObject[]}
 */
function findDuplicatedPrimaryKeyAndType(dapObjects) {
  const typeAndPrimaryKey = dapObjects.map(dapObject => `${dapObject.getType()}:${dapObject.getPrimaryKey()}`);

  const duplicates = findDuplicates(typeAndPrimaryKey);

  return duplicates.map((duplicate) => {
    const [type, primaryKey] = duplicate.split(':');

    // eslint-disable-next-line arrow-body-style
    return dapObjects.find((dapObject) => {
      return dapObject.getType() === type && dapObject.getPrimaryKey() === primaryKey;
    });
  });
}

module.exports = findDuplicatedPrimaryKeyAndType;
