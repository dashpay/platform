const StateTransitionHeader = require('../../../lib/blockchain/StateTransitionHeader');
const addSTPacketFactory = require('../../../lib/storage/ipfs/addSTPacketFactory');
const startIPFSInstance = require('../../../lib/test/services/mocha/startIPFSInstance');
const getTransitionPacketFixtures = require('../../../lib/test/fixtures/getTransitionPacketFixtures');
const getTransitionHeaderFixtures = require('../../../lib/test/fixtures/getTransitionHeaderFixtures');
const hashDataMerkleRoot = require('../../../lib/test/consensus/hashDataMerkleRoot');

describe('StateTransitionHeader', () => {
  const packet = getTransitionPacketFixtures()[0].toJSON();
  const header = getTransitionHeaderFixtures()[0].toJSON();

  let addSTPacket;
  startIPFSInstance().then((instance) => {
    addSTPacket = addSTPacketFactory(instance.getApi());
  });

  it('should StateTransitionHeader CID equal to IPFS CID', async () => {
    header.hashDataMerkleRoot = await hashDataMerkleRoot(packet);
    const stHeader = new StateTransitionHeader(header);

    const stHeaderCid = stHeader.getPacketCID();
    const ipfsCid = await addSTPacket(packet);

    expect(stHeaderCid).to.equal(ipfsCid);
  });
});
