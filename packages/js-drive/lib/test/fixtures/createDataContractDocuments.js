const dataContractMetaSchema = require('@dashevo/dpp/schema/dataContract/dataContractMeta.json');
const createIndices = require('./createIndices');
const createProperties = require('./createProperties');

/**
 *
 * @param {number} count
 * @returns {Object}
 */
function createDataContractDocuments(count = 2) {
  const documents = {};

  for (let i = 0; i < count; i++) {
    documents[`doc${i}`] = {
      type: 'object',
      indices: createIndices(dataContractMetaSchema.$defs.documentProperties.maxProperties, true),
      properties: createProperties(dataContractMetaSchema.$defs.documentProperties.maxProperties, {
        type: 'string',
        maxLength: 63,
      }),
      additionalProperties: false,
    };
  }

  return documents;
}

module.exports = createDataContractDocuments;
