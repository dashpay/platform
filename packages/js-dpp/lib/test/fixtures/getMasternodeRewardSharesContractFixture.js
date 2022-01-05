const masternodeRewardSharesDocuments = require('@dashevo/masternode-reward-shares-contract/schema/masternode-reward-shares-documents.json');
const DataContractFactory = require('../../dataContract/DataContractFactory');
const createDPPMock = require('../mocks/createDPPMock');
const generateRandomIdentifier = require('../utils/generateRandomIdentifier');

const ownerId = generateRandomIdentifier();

/**
 * @return {DataContract}
 */
module.exports = function getMasternodeRewardSharesContractFixture() {
  const factory = new DataContractFactory(createDPPMock(), () => {});

  return factory.create(ownerId, masternodeRewardSharesDocuments);
};
