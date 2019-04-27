const STPacket = require('../../stPacket/STPacket');

const getContractFixture = require('./getContractFixture');
const getDocumentsFixture = require('./getDocumentsFixture');

/**
 * @return {STPacket}
 */
function getSTPacketFixture() {
  const contract = getContractFixture();
  const documents = getDocumentsFixture()
    .map(document => document.removeMetadata());

  return new STPacket(contract.getId(), documents);
}

module.exports = getSTPacketFixture;
