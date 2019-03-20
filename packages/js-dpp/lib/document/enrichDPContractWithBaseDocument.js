const documentBaseSchema = require('../../schema/base/document');

/**
 * @typedef {enrichDPContractWithBaseDocument}
 * @param {DPContract} dpContract
 * @return {Object}
 */
function enrichDPContractWithBaseDocument(dpContract) {
  const rawDPContract = dpContract.toJSON();

  const jsonDPContract = JSON.stringify(rawDPContract);
  const clonedDPContract = JSON.parse(jsonDPContract);

  delete clonedDPContract.$schema;

  const { documents: clonedDocuments } = clonedDPContract;

  Object.keys(clonedDocuments).forEach((type) => {
    const clonedDocument = clonedDocuments[type];

    const { properties: baseDocumentProperties } = documentBaseSchema;

    if (!clonedDocument.required) {
      clonedDocument.required = [];
    }

    Object.keys(baseDocumentProperties).forEach((name) => {
      clonedDocument.properties[name] = baseDocumentProperties[name];
      clonedDocument.required.push(name);
    });
  });

  return clonedDPContract;
}

module.exports = enrichDPContractWithBaseDocument;
