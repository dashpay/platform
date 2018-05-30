const util = require('util');
const cbor = require('cbor');
const multihashingAsync = require('multihashing-async');
const multihashes = require('multihashes');
const StateTransitionHeader = require('../../../lib/blockchain/StateTransitionHeader');
const addSTPacketFactory = require('../../../lib/storage/addSTPacketFactory');
const startIPFSInstance = require('../../../lib/test/services/mocha/startIPFSInstance');
const getTransitionPacketFixtures = require('../../../lib/test/fixtures/getTransitionPacketFixtures');
const getTransitionHeaderFixtures = require('../../../lib/test/fixtures/getTransitionHeaderFixtures');

const multihashing = util.promisify(multihashingAsync);

async function hashDataMerkleRoot(packet) {
  const serializedPacket = cbor.encodeCanonical(packet);
  const multihash = await multihashing(serializedPacket, 'sha2-256');
  const decoded = multihashes.decode(multihash);
  return decoded.digest.toString('hex');
}

describe('StateTransitionHeader', () => {
  const packet = getTransitionPacketFixtures()[0].toJSON();
  const header = getTransitionHeaderFixtures()[0].toJSON();

  let addSTPacket;
  startIPFSInstance().then((instance) => {
    addSTPacket = addSTPacketFactory(instance.getApi());
  });

  it('should StateTransitionHeader CID equal to IPFS CID', async () => {
    header.hashDataMerkleRoot = await hashDataMerkleRoot(packet);
    const stHeader = new StateTransitionHeader(header);

    const stHeaderCid = stHeader.getPacketCID();
    const ipfsCid = await addSTPacket(packet);

    expect(stHeaderCid).to.equal(ipfsCid);
  });
});
