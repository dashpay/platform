const STPacket = require('../../stPacket/STPacket');

const getDapContractFixture = require('./getDapContractFixture');
const getDapObjectsFixture = require('./getDapObjectsFixture');

/**
 * @return {STPacket}
 */
function getSTPacketFixture() {
  const dapContract = getDapContractFixture();
  const dapObjects = getDapObjectsFixture();

  const stPacket = new STPacket(dapContract.getId(), dapObjects);

  stPacket.setItemsMerkleRoot('6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b')
    .setItemsHash('y90b273ff34fce19d6b804eff5a3f5747ada4eaa22f86fj5jf652ddb78755642');

  return stPacket;
}

module.exports = getSTPacketFixture;
