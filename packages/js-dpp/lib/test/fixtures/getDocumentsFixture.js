const getDataContractFixture = require('./getDataContractFixture');

const DocumentFactory = require('../../document/DocumentFactory');

const generateRandomId = require('../utils/generateRandomId');

const ownerId = generateRandomId();
const dataContract = getDataContractFixture();

/**
 * @return {Document[]}
 */
module.exports = function getDocumentsFixture() {
  const factory = new DocumentFactory(
    () => {},
    () => {},
  );

  return [
    factory.create(dataContract, ownerId, 'niceDocument', { name: 'Cutie' }),
    factory.create(dataContract, ownerId, 'prettyDocument', { lastName: 'Shiny' }),
    factory.create(dataContract, ownerId, 'prettyDocument', { lastName: 'Sweety' }),
    factory.create(dataContract, ownerId, 'indexedDocument', { firstName: 'William', lastName: 'Birkin' }),
    factory.create(dataContract, ownerId, 'indexedDocument', { firstName: 'Leon', lastName: 'Kennedy' }),
    factory.create(dataContract, ownerId, 'noTimeDocument', { name: 'ImOutOfTime' }),
    factory.create(dataContract, ownerId, 'uniqueDates', { firstName: 'John' }),
    factory.create(dataContract, ownerId, 'indexedDocument', { firstName: 'Bill' }),
  ];
};

module.exports.ownerId = ownerId;
module.exports.dataContract = dataContract;
