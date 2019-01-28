const { mocha: { startIPFS } } = require('@dashevo/js-evo-services-ctl');

const DashPlatformProtocol = require('@dashevo/dpp');

const STPacketIpfsRepository = require('../../../../lib/storage/stPacket/STPacketIpfsRepository');
const addSTPacketFactory = require('../../../../lib/storage/stPacket/addSTPacketFactory');
const getSTPacketsFixture = require('../../../../lib/test/fixtures/getSTPacketsFixture');

describe('addSTPacket', () => {
  let ipfsApi;
  let addSTPacket;

  startIPFS().then((ipfs) => {
    ipfsApi = ipfs.getApi();
  });

  beforeEach(() => {
    const dpp = new DashPlatformProtocol();

    const stPacketRepository = new STPacketIpfsRepository(
      ipfsApi,
      dpp,
      1000,
    );
    addSTPacket = addSTPacketFactory(stPacketRepository);
  });

  it('should add packets to storage and returns hash', async () => {
    const packets = getSTPacketsFixture();
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

    const packetsData = packets.map(packet => packet.toJSON());

    expect(packetsData).to.deep.equal(packetFromIPFS);
  });
});
