const crypto = require('crypto');

const getDataContractFixture = require('./getDataContractFixture');

const DocumentFactory = require('../../document/DocumentFactory');

const generateRandomIdentifier = require('../utils/generateRandomIdentifier');
const createDPPMock = require('../mocks/createDPPMock');

const ownerId = generateRandomIdentifier();

/**
 * @param {DataContract} [dataContract]
 * @return {Document[]}
 */
module.exports = function getDocumentsFixture(dataContract = getDataContractFixture()) {
  const factory = new DocumentFactory(
    createDPPMock(),
    () => ({
      isValid: () => true,
    }),
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
    factory.create(dataContract, ownerId, 'indexedDocument', { firstName: 'Bill', lastName: 'Gates' }),
    factory.create(dataContract, ownerId, 'withByteArrays', { byteArrayField: crypto.randomBytes(10), identifierField: generateRandomIdentifier().toBuffer() }),
    factory.create(dataContract, ownerId, 'optionalUniqueIndexedDocument', { firstName: 'Jacques-Yves', lastName: 'Cousteau' }),
  ];
};

module.exports.ownerId = ownerId;
