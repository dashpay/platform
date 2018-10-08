const addSTPacketFactory = require('../../../lib/storage/ipfs/addSTPacketFactory');
const { mocha: { startIPFS } } = require('@dashevo/js-evo-services-ctl');
const getTransitionPacketFixtures = require('../../../lib/test/fixtures/getTransitionPacketFixtures');
const getTransitionHeaderFixtures = require('../../../lib/test/fixtures/getTransitionHeaderFixtures');

describe('StateTransitionHeader', () => {
  const packet = getTransitionPacketFixtures()[0];
  const header = getTransitionHeaderFixtures()[0];

  let addSTPacket;
  startIPFS().then((instance) => {
    addSTPacket = addSTPacketFactory(instance.getApi());
  });

  it('should StateTransitionHeader CID equal to IPFS CID', async () => {
    header.extraPayload.setHashSTPacket(packet.getHash());

    const ipfsCid = await addSTPacket(packet);

    const stHeaderCid = header.getPacketCID();
    expect(stHeaderCid.toBaseEncodedString()).to.equal(ipfsCid.toBaseEncodedString());
  });
});
