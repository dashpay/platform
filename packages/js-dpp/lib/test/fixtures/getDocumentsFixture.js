const crypto = require('crypto');

const getDataContractFixture = require('./getDataContractFixture');

const DocumentFactory = require('../../document/DocumentFactory');

const generateRandomId = require('../utils/generateRandomId');

const ownerId = generateRandomId();

/**
 * @param {DataContract} [dataContract]
 * @return {Document[]}
 */
module.exports = function getDocumentsFixture(dataContract = getDataContractFixture()) {
  const factory = new DocumentFactory(
    () => {},
    () => {},
  );

  return [
    factory.create(dataContract, ownerId.toBuffer(), 'niceDocument', { name: 'Cutie' }),
    factory.create(dataContract, ownerId.toBuffer(), 'prettyDocument', { lastName: 'Shiny' }),
    factory.create(dataContract, ownerId.toBuffer(), 'prettyDocument', { lastName: 'Sweety' }),
    factory.create(dataContract, ownerId.toBuffer(), 'indexedDocument', { firstName: 'William', lastName: 'Birkin' }),
    factory.create(dataContract, ownerId.toBuffer(), 'indexedDocument', { firstName: 'Leon', lastName: 'Kennedy' }),
    factory.create(dataContract, ownerId.toBuffer(), 'noTimeDocument', { name: 'ImOutOfTime' }),
    factory.create(dataContract, ownerId.toBuffer(), 'uniqueDates', { firstName: 'John' }),
    factory.create(dataContract, ownerId.toBuffer(), 'indexedDocument', { firstName: 'Bill', lastName: 'Gates' }),
    factory.create(dataContract, ownerId.toBuffer(), 'withContentEncoding', { base64Field: crypto.randomBytes(10), base58Field: crypto.randomBytes(10) }),
    factory.create(dataContract, ownerId.toBuffer(), 'optionalUniqueIndexedDocument', { firstName: 'Jacques-Yves', lastName: 'Cousteau' }),
  ];
};

module.exports.ownerId = ownerId;
