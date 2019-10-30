const DocumentFactory = require('@dashevo/dpp/lib/document/DocumentFactory');

const getDataContractFixture = require('./getDataContractFixture');

const userId = '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288';

/**
 * @return {Document[]}
 */
function getDocumentsFixture() {
  const contract = getDataContractFixture();

  const validateDocumentStub = () => {};
  const fetchAndValidateDataContractStub = () => {};

  const factory = new DocumentFactory(
    validateDocumentStub,
    fetchAndValidateDataContractStub,
  );

  return [
    factory.create(contract, userId, 'niceDocument', { name: 'Cutie' }),
    factory.create(contract, userId, 'prettyDocument', { lastName: 'Shiny' }),
    factory.create(contract, userId, 'prettyDocument', { lastName: 'Sweety' }),
  ];
}

module.exports = getDocumentsFixture;
module.exports.userId = userId;
