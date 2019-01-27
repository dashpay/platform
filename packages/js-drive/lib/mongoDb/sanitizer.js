const RecursiveIterator = require('recursive-iterator');

const INTERNAL_PREFIX = '__';

/**
 * @typedef sanitizer
 */
const sanitizer = {
  INTERNAL_PREFIX,

  /**
   * Sanitize data before inserting to MongoDB
   *
   * @param {Object} data
   * @return {Object}
   */
  sanitize(data) {
    const sanitizedData = JSON.parse(JSON.stringify(data));

    const iterator = new RecursiveIterator(sanitizedData, 0, false, Number.MAX_VALUE);
    for (const { parent, node, key } of iterator) {
      if (key.startsWith && key.startsWith('$')) {
        delete parent[key];
        parent[`${INTERNAL_PREFIX}${key}`] = node;
      }
    }

    return sanitizedData;
  },

  /**
   * Unsanitize data after obtaining from MongoDB
   *
   * @param {Object} data
   * @return {Object}
   */
  unsanitize(data) {
    const unsanitizedData = JSON.parse(JSON.stringify(data));

    const iterator = new RecursiveIterator(unsanitizedData, 0, false, Number.MAX_VALUE);
    for (const { parent, node, key } of iterator) {
      if (typeof key === 'string' && key.startsWith(INTERNAL_PREFIX)) {
        delete parent[key];
        parent[key.substr(INTERNAL_PREFIX.length)] = node;
      }
    }

    return unsanitizedData;
  },
};

module.exports = sanitizer;
