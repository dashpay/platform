/**
 * @param {number} count
 * @param {Object} subSchema
 */
function createProperties(count, subSchema) {
  const properties = {};

  for (let i = 0; i < count; i++) {
    const name = `property${i}`;

    properties[name] = subSchema;
  }

  return properties;
}

module.exports = createProperties;
