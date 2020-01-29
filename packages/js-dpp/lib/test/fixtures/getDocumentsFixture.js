const getDataContractFixture = require('./getDataContractFixture');

const DocumentFactory = require('../../document/DocumentFactory');

const generateRandomId = require('../utils/generateRandomId');

const userId = generateRandomId();

/**
 * @return {Document[]}
 */
module.exports = function getDocumentsFixture() {
  const dataContract = getDataContractFixture();

  const factory = new DocumentFactory(
    () => {},
    () => {},
  );

  return [
    factory.create(dataContract, userId, 'niceDocument', { name: 'Cutie' }),
    factory.create(dataContract, userId, 'prettyDocument', { lastName: 'Shiny' }),
    factory.create(dataContract, userId, 'prettyDocument', { lastName: 'Sweety' }),
    factory.create(dataContract, userId, 'indexedDocument', { firstName: 'William', lastName: 'Birkin' }),
    factory.create(dataContract, userId, 'indexedDocument', { firstName: 'Leon', lastName: 'Kennedy' }),
  ];
};

module.exports.userId = userId;
