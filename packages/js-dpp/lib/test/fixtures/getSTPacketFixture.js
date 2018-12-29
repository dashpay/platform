const STPacket = require('../../stPacket/STPacket');

const getDapContractFixture = require('./getDapContractFixture');
const getDapObjectsFixture = require('./getDapObjectsFixture');

/**
 * @return {STPacket}
 */
function getSTPacketFixture() {
  const dapContract = getDapContractFixture();
  const dapObjects = getDapObjectsFixture();

  return new STPacket(dapContract.getId(), dapObjects);
}

module.exports = getSTPacketFixture;
