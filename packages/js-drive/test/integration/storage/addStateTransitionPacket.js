const addStateTransitionPacket = require('../../../lib/storage/addStateTransitionPacket');
const StateTransitionPacket = require('../../../lib/storage/StateTransitionPacket');

const startIPFSInstance = require('../../../lib/test/services/IPFS/startIPFSInstance');

const getStateTransitionPackets = require('../../fixtures/getStateTransitionPackets');

describe('addStateTransitionPacket', () => {
  let ipfsApi;
  before(async () => {
    ipfsApi = await startIPFSInstance();
  });

  it('should add packets to storage and returns hash', async () => {
    const packetsData = getStateTransitionPackets();
    const packets = packetsData.map(packetData => new StateTransitionPacket(packetData));
    const addPacketsPromises = packets.map(addStateTransitionPacket.bind(null, ipfsApi));
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
