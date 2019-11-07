/**
 * Extract indexed fields from document schema
 *
 * @typedef getIndexedFieldsFromDocumentSchema
 * @param {Object} documentSchema
 * @returns {Array}
 */
function getIndexedFieldsFromDocumentSchema(documentSchema) {
  const indexFields = documentSchema.indices || [];

  return indexFields
    .map(({ properties }) => properties)
    // add system fields
    .concat([[
      { $id: 'asc' },
    ], [
      { $id: 'desc' },
    ]]);
}

module.exports = getIndexedFieldsFromDocumentSchema;
