const DocumentFactory = require('@dashevo/dpp/lib/document/DocumentFactory');

const getContractFixture = require('./getContractFixture');

const userId = '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288';

/**
 * @return {Document[]}
 */
function getDocumentsFixture() {
  const contract = getContractFixture();

  const validateDocumentStub = () => {};

  const factory = new DocumentFactory(
    userId,
    contract,
    validateDocumentStub,
  );

  return [
    factory.create('niceDocument', { name: 'Cutie' }),
    factory.create('prettyDocument', { lastName: 'Shiny' }),
    factory.create('prettyDocument', { lastName: 'Sweety' }),
  ];
}

module.exports = getDocumentsFixture;
module.exports.userId = userId;
