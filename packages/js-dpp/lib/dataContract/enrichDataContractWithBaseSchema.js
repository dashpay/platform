const lodashCloneDeep = require('lodash.clonedeep');

const DataContract = require('./DataContract');

/**
 * @typedef {enrichDataContractWithBaseSchema}
 *
 * @param {DataContract|RawDataContract} dataContract
 * @param {Object} baseSchema
 * @param {string} [schemaIdPrefix='']
 * @param {string[]} [excludeProperties]
 *
 * @return {DataContract}
 */
function enrichDataContractWithBaseSchema(
  dataContract,
  baseSchema,
  schemaIdPrefix = '',
  excludeProperties = [],
) {
  const rawDataContract = (dataContract instanceof DataContract)
    ? dataContract.toJSON()
    : dataContract;

  const clonedDataContract = lodashCloneDeep(rawDataContract);

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

  // Add schema $id prefix
  clonedDataContract.$id = schemaIdPrefix + clonedDataContract.$id;

  return new DataContract(clonedDataContract);
}

module.exports = enrichDataContractWithBaseSchema;
