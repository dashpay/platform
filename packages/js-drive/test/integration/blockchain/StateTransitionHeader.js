const StateTransitionHeader = require('../../../lib/blockchain/StateTransitionHeader');
const addSTPacketFactory = require('../../../lib/storage/ipfs/addSTPacketFactory');
const startIPFSInstance = require('../../../lib/test/services/mocha/startIPFSInstance');
const getTransitionPacketFixtures = require('../../../lib/test/fixtures/getTransitionPacketFixtures');
const getTransitionHeaderFixtures = require('../../../lib/test/fixtures/getTransitionHeaderFixtures');
const hashSTPacket = require('../../../lib/test/consensus/hashSTPacket');

describe('StateTransitionHeader', () => {
  const packet = getTransitionPacketFixtures()[0];
  const header = getTransitionHeaderFixtures()[0].toJSON();

  let addSTPacket;
  startIPFSInstance().then((instance) => {
    addSTPacket = addSTPacketFactory(instance.getApi());
  });

  it('should StateTransitionHeader CID equal to IPFS CID', async () => {
    header.hashSTPacket = await hashSTPacket(packet.toJSON({ skipMeta: true }));
    const stHeader = new StateTransitionHeader(header);

    const stHeaderCid = stHeader.getPacketCID();
    const ipfsCid = await addSTPacket(packet);

    expect(stHeaderCid).to.equal(ipfsCid);
  });
});
