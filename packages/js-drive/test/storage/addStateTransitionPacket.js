const fs = require('fs');
const path = require('path');
const { expect } = require('chai');
const cbor = require('cbor');

const addStateTransitionPacket = require('../../lib/storage/addStateTransitionPacket');
const StateTransitionPacket = require('../../lib/storage/StateTransitionPacket');

const startIPFSInstance = require('../../lib/test/startIPFSInstance');

describe('addStateTransitionPacket', () => {
  let ipfsApi;

  startIPFSInstance().then((_ipfsApi) => {
    ipfsApi = _ipfsApi;
  });

  it('should add packets to storage and returns hash', async () => {
    // TODO: extract to separate method
    const packetsJSON = fs.readFileSync(path.join(__dirname, '/../fixtures/stateTransitionPackets.json'));
    const packetsData = JSON.parse(packetsJSON);

    const packets = packetsData.map(packetData => new StateTransitionPacket(packetData));

    const addPacketPromises = packets.map(addStateTransitionPacket.bind(null, ipfsApi));

    const packetMultihashes = await Promise.all(addPacketPromises);

    // 1. Packets should be available in IPFS
    // eslint-disable-next-line arrow-body-style
    const packetPromisesFromIPFS = packetMultihashes.map((packetMultihash) => {
      return ipfsApi.object.get(packetMultihash);
    });

    const packetsFromIPFS = await Promise.all(packetPromisesFromIPFS);

    // 2. Packets should have the same data
    const packetDataFromIPFS = packetsFromIPFS.map(packet => JSON.parse(packet.data));
    const packetData = packets.map((packet) => {
      const data = Object.assign({}, packet.data);
      delete data.objects;
      return data;
    });

    expect(packetData).to.deep.equal(packetDataFromIPFS);

    // 3. Objects contained in corresponding packets should be available in IPFS with correct data
    for (const [i, packet] of packetsFromIPFS.entries()) {
      // eslint-disable-next-line no-loop-func
      const objectPromisesFromIPFS = packet.links.map(link => ipfsApi.block.get(link.multihash));

      const objectsFromIPFS = await Promise.all(objectPromisesFromIPFS);

      const objectDataFromIPFS = objectsFromIPFS.map(object => cbor.decodeFirstSync(object.data));

      const objectData = packets[i].data.objects.map(object => object.data);

      expect(objectDataFromIPFS).to.deep.equal(objectData);
    }
  });
});
