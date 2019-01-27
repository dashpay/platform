const STPacket = require('@dashevo/dpp/lib/stPacket/STPacket');

const getDPContractFixture = require('./getDPContractFixture');
const getDPObjectsFixture = require('./getDPObjectsFixture');

/**
 * @return {STPacket[]}
 */
function getSTPacketsFixture() {
  const dpContract = getDPContractFixture();
  const dpObjects = getDPObjectsFixture();

  const contractId = dpContract.getId();

  return [
    new STPacket(contractId, dpContract),
    new STPacket(contractId, dpObjects.slice(0, 2)),
    new STPacket(contractId, dpObjects.slice(2)),
  ];
}

module.exports = getSTPacketsFixture;
