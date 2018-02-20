const util = require('util');
const cbor = require('cbor');
const CID = require('cids');

const multihashingAsync = require('multihashing-async');

const multihashing = util.promisify(multihashingAsync);

/**
 * Generate ST object base encoded IPFS multihash
 *
 * @param object ST object
 * @return {Promise<string>}
 */
module.exports = async function generateObjectMultihash(object) {
  const serializedData = cbor.encode(object.data);
  const objectMultihash = await multihashing(serializedData, 'sha2-256');

  const cid = new CID(0, 'dag-pb', objectMultihash);

  return cid.toBaseEncodedString();
};
