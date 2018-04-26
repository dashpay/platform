const fs = require('fs');
const path = require('path');

const addStateTransitionPacket = require('../../../../lib/storage/addStateTransitionPacket');
const StateTransitionPacket = require('../../../../lib/storage/StateTransitionPacket');
const generatePacketMultihash = require('../../../../lib/storage/ipfs/generatePacketMultihash');

const startIPFSInstance = require('../../../../lib/test/services/IPFS/startIPFSInstance');

describe('generatePacketMultihash', () => {
  let ipfsApi;

  startIPFSInstance().then((_ipfsApi) => {
    ipfsApi = _ipfsApi;
  });

  it('should generate the same multihash as IPFS', async () => {
    const packetsJSON = fs.readFileSync(path.join(__dirname, '/../../../fixtures/stateTransitionPackets.json'));
    const packetsData = JSON.parse(packetsJSON);

    const packets = packetsData.map(packetData => new StateTransitionPacket(packetData));

    // Add packets from fixtures to IPFS
    const addPacketPromises = packets.map(addStateTransitionPacket.bind(null, ipfsApi));

    const packetMultihashesFromIPFS = await Promise.all(addPacketPromises);

    // Generate IPFS multihashes from fixtures' packets
    const generateMultihashesPromises = packets.map(generatePacketMultihash);

    const packetMultihashes = await Promise.all(generateMultihashesPromises);

    expect(packetMultihashes).to.be.deep.equal(packetMultihashesFromIPFS);
  });
});
