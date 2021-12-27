const DocumentFactory = require('../../document/DocumentFactory');
const createDPPMock = require('../mocks/createDPPMock');
const generateRandomIdentifier = require('../utils/generateRandomIdentifier');
const getRewardShareContractFixture = require('./getRewardShareContractFixture');

const ownerId = generateRandomIdentifier();
const payToId = generateRandomIdentifier();
const dataContract = getRewardShareContractFixture();

function getRewardShareDocumentsFixture() {
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

module.exports = getRewardShareDocumentsFixture;

module.exports.dataContract = dataContract;
