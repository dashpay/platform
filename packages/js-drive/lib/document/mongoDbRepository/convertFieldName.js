const systemFields = {
  $id: '_id',
  $ownerId: 'ownerId',
  $revision: 'revision',
  $createdAt: 'createdAt',
  $updatedAt: 'updatedAt',
  $protocolVersion: 'protocolVersion',
};

/**
 * Convert field name to MongoDB internal representation
 *
 * @param {string} field
 * @return {string}
 */
function convertFieldName(field) {
  if (!field.startsWith('$')) {
    return `data.${field}`;
  }

  if (!systemFields[field]) {
    throw new Error(`System field ${field} is not defined`);
  }

  return systemFields[field];
}

module.exports = convertFieldName;
