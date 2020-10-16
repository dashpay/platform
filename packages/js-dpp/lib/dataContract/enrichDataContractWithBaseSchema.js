const lodashCloneDeep = require('lodash.clonedeep');

const DataContract = require('./DataContract');

/**
 * @typedef {enrichDataContractWithBaseSchema}
 *
 * @param {DataContract} dataContract
 * @param {Object} baseSchema
 * @param {number} schemaIdBytePrefix
 * @param {string[]} [excludeProperties]
 *
 * @return {DataContract}
 */
function enrichDataContractWithBaseSchema(
  dataContract,
  baseSchema,
  schemaIdBytePrefix,
  excludeProperties = [],
) {
  const clonedDataContract = lodashCloneDeep(dataContract.toObject());

  delete clonedDataContract.$schema;

  const { documents: clonedDocuments } = clonedDataContract;

  Object.keys(clonedDocuments).forEach((type) => {
    const clonedDocument = clonedDocuments[type];

    const {
      properties: baseProperties,
      required: baseRequired,
    } = baseSchema;

    if (!clonedDocument.required) {
      clonedDocument.required = [];
    }

    Object.keys(baseProperties)
      .forEach((name) => {
        clonedDocument.properties[name] = baseProperties[name];
      });

    baseRequired.forEach((name) => clonedDocument.required.push(name));

    excludeProperties.forEach((property) => {
      delete clonedDocument[property];
    });

    clonedDocument.required = clonedDocument.required
      .filter((property) => !excludeProperties.includes(property));
  });

  // Ajv caches schemas using $id internally
  // so we can't pass two different schemas with the same $id.
  // Hacky solution for that is to replace first four bytes
  // in $id with passed prefix byte
  clonedDataContract.$id[0] = schemaIdBytePrefix;
  clonedDataContract.$id[1] = schemaIdBytePrefix;
  clonedDataContract.$id[2] = schemaIdBytePrefix;
  clonedDataContract.$id[4] = schemaIdBytePrefix;

  return new DataContract(clonedDataContract);
}

enrichDataContractWithBaseSchema.PREFIX_BYTE_0 = 0;
enrichDataContractWithBaseSchema.PREFIX_BYTE_1 = 1;
enrichDataContractWithBaseSchema.PREFIX_BYTE_2 = 2;
enrichDataContractWithBaseSchema.PREFIX_BYTE_3 = 3;

module.exports = enrichDataContractWithBaseSchema;
