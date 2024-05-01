const crypto = require('crypto');

const getDataContractFixture = require('./getDataContractFixture');

const { default: loadWasmDpp } = require('../../..');
let { DocumentFactory } = require('../../..');
const generateRandomIdentifierAsync = require('../utils/generateRandomIdentifierAsync');

let ownerId;

/**
 * @param {DataContract} [dataContract]
 * @return {Promise<Document[]>}
 */
module.exports = async function getDocumentsFixture(
  dataContract,
) {
  if (!dataContract) {
    // eslint-disable-next-line no-param-reassign
    dataContract = await getDataContractFixture();
  }

  ({ DocumentFactory } = await loadWasmDpp());

  // const documentValidator = new DocumentValidator(new ProtocolVersionValidator());
  const entropyGenerator = {
    generate() {
      return crypto.randomBytes(32);
    },
  };
  const factory = new DocumentFactory(1, entropyGenerator);

  if (!ownerId) {
    ownerId = await generateRandomIdentifierAsync();
  }

  return [
    factory.create(dataContract, ownerId, 'niceDocument', { name: 'Cutie' }),
    factory.create(dataContract, ownerId, 'prettyDocument', { lastName: 'Shiny' }),
    factory.create(dataContract, ownerId, 'prettyDocument', { lastName: 'Sweety' }),
    factory.create(dataContract, ownerId, 'indexedDocument', { firstName: 'William', lastName: 'Birkin' }),
    factory.create(dataContract, ownerId, 'indexedDocument', { firstName: 'Leon', lastName: 'Kennedy' }),
    factory.create(dataContract, ownerId, 'noTimeDocument', { name: 'ImOutOfTime' }),
    factory.create(dataContract, ownerId, 'uniqueDates', { firstName: 'John' }),
    factory.create(dataContract, ownerId, 'indexedDocument', { firstName: 'Bill', lastName: 'Gates' }),
    factory.create(dataContract, ownerId, 'withByteArrays', {
      byteArrayField: crypto.randomBytes(10),
      identifierField: (await generateRandomIdentifierAsync()).toBuffer(),
    }),
    factory.create(dataContract, ownerId, 'optionalUniqueIndexedDocument', { firstName: 'Jacques-Yves', lastName: 'Cousteau' }),
  ];
};
