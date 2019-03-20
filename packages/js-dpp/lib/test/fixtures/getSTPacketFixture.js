const STPacket = require('../../stPacket/STPacket');

const getDPContractFixture = require('./getDPContractFixture');
const getDocumentsFixture = require('./getDocumentsFixture');

/**
 * @return {STPacket}
 */
function getSTPacketFixture() {
  const dpContract = getDPContractFixture();
  const documents = getDocumentsFixture();

  return new STPacket(dpContract.getId(), documents);
}

module.exports = getSTPacketFixture;
