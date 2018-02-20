const generateObjectCid = require('./generateObjectMultihash');
const createPacketDagNode = require('./createPacketDagNode');

/**
 * Generate ST packet base encoded IPFS multihash
 *
 * @param {StateTransitionPacket} packet
 * @return {Promise<string>}
 */
module.exports = async function generatePacketMultihash(packet) {
  const packetData = Object.assign({}, packet.data);
  const { objects } = packetData;
  delete packetData.objects;

  const objectsCidPromises = objects.map(generateObjectCid);

  const objectCids = await Promise.all(objectsCidPromises);

  const packetDagNode = await createPacketDagNode(packetData, objectCids);

  return packetDagNode.toJSON().multihash;
};
