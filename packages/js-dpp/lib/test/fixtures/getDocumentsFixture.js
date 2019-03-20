const getDPContractFixture = require('./getDPContractFixture');

const DocumentFactory = require('../../document/DocumentFactory');

const userId = '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288';

/**
 * @return {Document[]}
 */
module.exports = function getDocumentsFixture() {
  const dpContract = getDPContractFixture();

  const validateDocumentStub = () => {};

  const factory = new DocumentFactory(
    userId,
    dpContract,
    validateDocumentStub,
  );

  return [
    factory.create('niceDocument', { name: 'Cutie' }),
    factory.create('prettyDocument', { lastName: 'Shiny' }),
    factory.create('prettyDocument', { lastName: 'Sweety' }),
    factory.create('indexedDocument', { firstName: 'William', lastName: 'Birkin' }),
    factory.create('indexedDocument', { firstName: 'Leon', lastName: 'Kennedy' }),
  ];
};

module.exports.userId = userId;
