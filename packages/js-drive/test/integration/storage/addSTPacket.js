const addSTPacketFactory = require('../../../lib/storage/addSTPacketFactory');
const StateTransitionPacket = require('../../../lib/storage/StateTransitionPacket');

const startIPFSInstance = require('../../../lib/test/services/IPFS/startIPFSInstance');

const getStateTransitionPackets = require('../../fixtures/getStateTransitionPackets');

describe('addSTPacket', () => {
  let ipfsApi;
  let addSTPacket;

  before(async function before() {
    this.timeout(25000);
    ipfsApi = await startIPFSInstance();
    addSTPacket = addSTPacketFactory(ipfsApi);
  });

  it('should add packets to storage and returns hash', async () => {
    const packetsData = getStateTransitionPackets();
    const packets = packetsData.map(packetData => new StateTransitionPacket(packetData));
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

    expect(packets).to.deep.equal(packetFromIPFS);
  });
});
