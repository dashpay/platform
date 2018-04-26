const fs = require('fs');
const path = require('path');
const cbor = require('cbor');
const crypto = require('crypto');
const mh = require('multihashes');

const generateObjectMultihash = require('../../../../lib/storage/ipfs/generateObjectMultihash');

const startIPFSInstance = require('../../../../lib/test/services/IPFS/startIPFSInstance');

const packetsJSON = fs.readFileSync(path.join(__dirname, '/../../../fixtures/stateTransitionPackets.json'));
const packetsData = JSON.parse(packetsJSON);
const object = packetsData[0].data.objects[0];


describe('generateObjectMultihash', () => {
  let ipfsApi;
  let multihash;

  startIPFSInstance().then((_ipfsApi) => {
    ipfsApi = _ipfsApi;
  });

  beforeEach(async () => {
    multihash = await generateObjectMultihash(object);
  });

  it('should generate the same multihash as IPFS', async () => {
    const objectFromIPFS = await ipfsApi.block.put(cbor.encode(object.data));

    expect(objectFromIPFS.cid.toBaseEncodedString()).to.be.equal(multihash);
  });

  it('should generate multihash which should contain sha256 of object data', () => {
    // Extract sha256 digest from object multihash
    const multihashBuffer = mh.fromB58String(multihash);
    const decodedMultihash = mh.decode(multihashBuffer);
    const digestFromMultihash = decodedMultihash.digest.toString('hex');

    // Get sha256 of object data
    const serializedObject = cbor.encode(object.data);
    const hash = crypto.createHash('sha256');
    hash.update(serializedObject);
    const objectDigest = hash.digest().toString('hex');

    expect(digestFromMultihash).to.be.equal(objectDigest);
  });
});

