const { mocha: { startIPFS } } = require('@dashevo/js-evo-services-ctl');
const addSTPacketFactory = require('../../../lib/storage/stPacket/addSTPacketFactory');
const StateTransitionPacketIpfsRepository = require('../../../lib/storage/stPacket/StateTransitionPacketIpfsRepository');
const getTransitionPacketFixtures = require('../../../lib/test/fixtures/getTransitionPacketFixtures');
const getTransitionHeaderFixtures = require('../../../lib/test/fixtures/getTransitionHeaderFixtures');

describe('StateTransitionHeader', () => {
  const packet = getTransitionPacketFixtures()[0];
  const header = getTransitionHeaderFixtures()[0];

  let addSTPacket;
  startIPFS().then((instance) => {
    const stPacketRepository = new StateTransitionPacketIpfsRepository(
      instance.getApi(),
      1000,
    );
    addSTPacket = addSTPacketFactory(stPacketRepository);
  });

  it('should StateTransitionHeader CID equal to IPFS CID', async () => {
    header.extraPayload.setHashSTPacket(packet.getHash());

    const ipfsCid = await addSTPacket(packet);

    const stHeaderCid = header.getPacketCID();
    expect(stHeaderCid.toBaseEncodedString()).to.equal(ipfsCid.toBaseEncodedString());
  });
});
