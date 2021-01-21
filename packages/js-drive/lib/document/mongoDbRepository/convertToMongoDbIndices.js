const convertFieldName = require('./convertFieldName');

/**
 * Convert indices from contract format to mongoDB format
 *
 * @typedef convertToMongoDbIndices
 * @param {Array} indices
 * @returns {Array}
 */
function convertToMongoDbIndices(indices) {
  return indices.map((index) => {
    const key = index.properties.reduce((result, item) => {
      const [[field, order]] = Object.entries(item);
      const newProperty = {
        [convertFieldName(field)]: order.toLowerCase() === 'asc' ? 1 : -1,
      };

      return { ...result, ...newProperty };
    }, {});

    return {
      key,
      unique: !!index.unique,
    };
  });
}

module.exports = convertToMongoDbIndices;
