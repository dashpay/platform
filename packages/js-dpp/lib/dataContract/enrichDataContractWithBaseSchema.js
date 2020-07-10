const DataContract = require('./DataContract');

/**
 * @typedef {enrichDataContractWithBaseSchema}
 *
 * @param {DataContract|RawDataContract} dataContract
 * @param {Object} baseSchema
 * @param {string[]} [excludeProperties]
 *
 * @return {RawDataContract}
 */
function enrichDataContractWithBaseSchema(
  dataContract,
  baseSchema,
  excludeProperties = [],
) {
  const rawDataContract = (dataContract instanceof DataContract)
    ? dataContract.toJSON()
    : dataContract;

  const jsonDataContract = JSON.stringify(rawDataContract);
  const clonedDataContract = JSON.parse(jsonDataContract);

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

  return clonedDataContract;
}

module.exports = enrichDataContractWithBaseSchema;
