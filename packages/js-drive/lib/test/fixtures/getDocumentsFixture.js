const DocumentFactory = require('@dashevo/dpp/lib/document/DocumentFactory');

const generateRandomId = require('@dashevo/dpp/lib/test/utils/generateRandomId');

const getDataContractFixture = require('./getDataContractFixture');

const contract = getDataContractFixture();

const userId = generateRandomId();

/**
 * @return {Document[]}
 */
function getDocumentsFixture() {
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
