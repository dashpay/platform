const DocumentFactory = require('../../document/DocumentFactory');
const createDPPMock = require('../mocks/createDPPMock');
const generateRandomIdentifier = require('../utils/generateRandomIdentifier');
const getMasternodeRewardSharesContractFixture = require('./getMasternodeRewardSharesContractFixture');

const ownerId = generateRandomIdentifier();
const payToId = generateRandomIdentifier();
const dataContract = getMasternodeRewardSharesContractFixture();

function getMasternodeRewardShareDocumentsFixture() {
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

module.exports.dataContract = dataContract;
