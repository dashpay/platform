const DocumentFactory = require('../../document/DocumentFactory');
const createDPPMock = require('../mocks/createDPPMock');
const generateRandomIdentifier = require('../utils/generateRandomIdentifier');
const getMasternodeRewardSharesContractFixture = require('./getMasternodeRewardSharesContractFixture');

function getMasternodeRewardShareDocumentsFixture(
  ownerId = generateRandomIdentifier(),
  payToId = generateRandomIdentifier(),
  dataContract = getMasternodeRewardSharesContractFixture(),
) {
  const factory = new DocumentFactory(
    createDPPMock(),
    () => ({
      isValid: () => true,
    }),
    () => {},
  );

  return [
    factory.create(dataContract, ownerId, 'rewardShare', {
      payToId,
      percentage: 500,
    }),
  ];
}

module.exports = getMasternodeRewardShareDocumentsFixture;
