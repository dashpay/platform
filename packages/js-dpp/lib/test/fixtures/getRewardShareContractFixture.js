const rewardShareDocuments = require('@dashevo/reward-share-contract/schema/reward-share-documents.json');
const DataContractFactory = require('../../dataContract/DataContractFactory');
const createDPPMock = require('../mocks/createDPPMock');
const generateRandomIdentifier = require('../utils/generateRandomIdentifier');

const ownerId = generateRandomIdentifier();

/**
 * @return {DataContract}
 */
module.exports = function getRewardShareContractFixture() {
  const factory = new DataContractFactory(createDPPMock(), () => {});

  return factory.create(ownerId, rewardShareDocuments);
};
