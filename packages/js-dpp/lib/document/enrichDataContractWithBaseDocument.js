const baseDocumentSchema = require('../../schema/base/document');

/**
 * @typedef {enrichDataContractWithBaseDocument}
 * @param {DataContract} dataContract
 * @param {string[]} excludeBaseDocumentProperties
 * @return {RawDataContract}
 */
function enrichDataContractWithBaseDocument(dataContract, excludeBaseDocumentProperties = []) {
  const rawDataContract = dataContract.toJSON();

  const jsonDataContract = JSON.stringify(rawDataContract);
  const clonedDataContract = JSON.parse(jsonDataContract);

  delete clonedDataContract.$schema;

  const { documents: clonedDocuments } = clonedDataContract;

  Object.keys(clonedDocuments).forEach((type) => {
    const clonedDocument = clonedDocuments[type];

    const {
      properties: baseDocumentProperties,
      required: baseDocumentRequired,
    } = baseDocumentSchema;

    if (!clonedDocument.required) {
      clonedDocument.required = [];
    }

    Object.keys(baseDocumentProperties)
      .filter(name => excludeBaseDocumentProperties.indexOf(name) === -1)
      .forEach((name) => {
        clonedDocument.properties[name] = baseDocumentProperties[name];
      });

    baseDocumentRequired.forEach(name => clonedDocument.required.push(name));
  });

  return clonedDataContract;
}

module.exports = enrichDataContractWithBaseDocument;
