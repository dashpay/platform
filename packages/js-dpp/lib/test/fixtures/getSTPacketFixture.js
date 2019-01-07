const STPacket = require('../../stPacket/STPacket');

const getDPContractFixture = require('./getDPContractFixture');
const getDPObjectsFixture = require('./getDPObjectsFixture');

/**
 * @return {STPacket}
 */
function getSTPacketFixture() {
  const dpContract = getDPContractFixture();
  const dpObjects = getDPObjectsFixture();

  return new STPacket(dpContract.getId(), dpObjects);
}

module.exports = getSTPacketFixture;
