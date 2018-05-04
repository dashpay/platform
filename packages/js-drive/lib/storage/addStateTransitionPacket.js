/**
 * Store State Transition packet in IPFS
 *
 * Stores and pins ST packet to IPFS storage and returns its hash
 *
 * @param {IpfsApi} ipfsApi IPFSApi instance
 * @param {StateTransitionPacket[]} packet State Transition packet
 * @return {Promise<string>}
 */
module.exports = async function addStateTransitionPacket(ipfsApi, packet) {
  const cid = await ipfsApi.dag.put(packet, { format: 'dag-cbor', hashAlg: 'sha2-256' });
  return cid.toBaseEncodedString();
};
