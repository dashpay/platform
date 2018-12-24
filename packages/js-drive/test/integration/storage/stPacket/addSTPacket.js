const { mocha: { startIPFS } } = require('@dashevo/js-evo-services-ctl');

const StateTransitionPacketIpfsRepository = require('../../../../lib/storage/stPacket/StateTransitionPacketIpfsRepository');
const addSTPacketFactory = require('../../../../lib/storage/stPacket/addSTPacketFactory');
const getTransitionPacketFixtures = require('../../../../lib/test/fixtures/getTransitionPacketFixtures');

describe('addSTPacket', () => {
  let ipfsApi;
  let addSTPacket;

  startIPFS().then((_instance) => {
    ipfsApi = _instance.getApi();
  });

  beforeEach(() => {
    const stPacketRepository = new StateTransitionPacketIpfsRepository(
      ipfsApi,
      1000,
    );
    addSTPacket = addSTPacketFactory(stPacketRepository);
  });

  it('should add packets to storage and returns hash', async () => {
    const packets = getTransitionPacketFixtures();
    const addPacketsPromises = packets.map(addSTPacket);
    const packetsCids = await Promise.all(addPacketsPromises);

    // 1. Packets should be available in IPFS
    // eslint-disable-next-line arrow-body-style
    const packetsPromisesFromIPFS = packetsCids.map((packetCid) => {
      return ipfsApi.dag.get(packetCid);
    });

    const packetsFromIPFS = await Promise.all(packetsPromisesFromIPFS);

    // 2. Packets should have the same data
    const packetFromIPFS = packetsFromIPFS.map(packet => packet.value);

    const packetsData = packets.map(packet => packet.toJSON({ skipMeta: true }));

    expect(packetsData).to.deep.equal(packetFromIPFS);
  });
});
