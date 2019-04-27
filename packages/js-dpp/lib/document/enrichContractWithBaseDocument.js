const baseDocumentSchema = require('../../schema/base/document');

/**
 * @typedef {enrichContractWithBaseDocument}
 * @param {Contract} contract
 * @param {string[]} excludeBaseDocumentProperties
 * @return {RawContract}
 */
function enrichContractWithBaseDocument(contract, excludeBaseDocumentProperties = []) {
  const rawContract = contract.toJSON();

  const jsonContract = JSON.stringify(rawContract);
  const clonedContract = JSON.parse(jsonContract);

  delete clonedContract.$schema;

  const { documents: clonedDocuments } = clonedContract;

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

  return clonedContract;
}

module.exports = enrichContractWithBaseDocument;
