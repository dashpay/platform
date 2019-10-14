const STPacket = require('../../stPacket/STPacket');

const getDataContractFixture = require('./getDataContractFixture');
const getDocumentsFixture = require('./getDocumentsFixture');

/**
 * @return {STPacket}
 */
function getSTPacketFixture() {
  const dataContract = getDataContractFixture();
  const documents = getDocumentsFixture();

  return new STPacket(dataContract.getId(), documents);
}

module.exports = getSTPacketFixture;
