const systemFields = {
  $id: '_id',
  $userId: 'userId',
};

/**
 * Convert field name to MongoDB internal representation
 *
 * @param {string} field
 * @return {string}
 */
function convertFieldName(field) {
  let mongoDbField = `document.${field}`;
  if (field.startsWith('$')) {
    mongoDbField = systemFields[field];
  }

  return mongoDbField;
}

module.exports = convertFieldName;
