const addSTPacketFactory = require('../../../../lib/storage/ipfs/addSTPacketFactory');

const startIPFSInstance = require('../../../../lib/test/services/mocha/startIPFSInstance');

const getTransitionPacketFixtures = require('../../../../lib/test/fixtures/getTransitionPacketFixtures');

describe('addSTPacket', () => {
  let ipfsApi;
  let addSTPacket;

  startIPFSInstance().then((_instance) => {
    ipfsApi = _instance.getApi();
    addSTPacket = addSTPacketFactory(ipfsApi);
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
