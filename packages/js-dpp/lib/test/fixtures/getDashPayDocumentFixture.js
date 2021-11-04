const getDashPayContractFixture = require('./getDashPayContractFixture');
const DocumentFactory = require('../../document/DocumentFactory');
const generateRandomIdentifier = require('../utils/generateRandomIdentifier');
const createDPPMock = require('../mocks/createDPPMock');

const ownerId = generateRandomIdentifier();

/**
 * @return {Document}
 */
function getContactRequestDocumentFixture(
  dataContract = getDashPayContractFixture(),
  options = {},
) {
  const factory = new DocumentFactory(
    createDPPMock(),
    () => ({
      isValid: () => true,
    }),
    () => {},
  );

  const data = {
    toUserId: Buffer.alloc(32),
    encryptedPublicKey: Buffer.alloc(96),
    senderKeyIndex: 0,
    recipientKeyIndex: 0,
    accountReference: 0,
    ...options,
  };

  return factory.create(dataContract, ownerId, 'contactRequest', data);
}

module.exports = {
  getContactRequestDocumentFixture,
};
