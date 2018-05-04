const util = require('util');
const cbor = require('cbor');
const multihashingAsync = require('multihashing-async');
const multihashes = require('multihashes');
const StateTransitionHeader = require('../../../lib/blockchain/StateTransitionHeader');
const addStateTransitionPacket = require('../../../lib/storage/addStateTransitionPacket');
const startIPFSInstance = require('../../../lib/test/services/IPFS/startIPFSInstance');
const getStateTransitionPackets = require('../../fixtures/getStateTransitionPackets');
const getStateTransitionHeaders = require('../../fixtures/getStateTransitionHeaders');

const multihashing = util.promisify(multihashingAsync);

async function hashDataMerkleRoot(packet) {
  const serializedPacket = cbor.encodeCanonical(packet);
  const multihash = await multihashing(serializedPacket, 'sha2-256');
  const decoded = multihashes.decode(multihash);
  return decoded.digest.toString('hex');
}

describe('StateTransitionHeader', () => {
  const packets = getStateTransitionPackets();
  const packet = packets[0];

  const headers = getStateTransitionHeaders();
  const header = headers[0];

  let ipfsApi;
  before(async () => {
    ipfsApi = await startIPFSInstance();
  });

  it('should StateTransitionHeader CID equal to IPFS CID', async () => {
    header.hashDataMerkleRoot = await hashDataMerkleRoot(packet);
    const stHeader = new StateTransitionHeader(header);

    const stHeaderCid = stHeader.getPacketCID();
    const ipfsCid = await addStateTransitionPacket(ipfsApi, packet);

    expect(stHeaderCid).to.equal(ipfsCid);
  });
});
