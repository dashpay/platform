const dagPB = require('ipld-dag-pb');
const util = require('util');

const createDagNode = util.promisify(dagPB.DAGNode.create);

/**
 * Create IPFS DAG node for ST packet
 *
 * @param {Object} packetData ST packet data
 * @param {Object[]} objectCids List of contained ST object IPFS CIDs
 * @return {Promise<dagPB.DAGNode>}
 */
module.exports = async function createPacketDagNode(packetData, objectCids) {
  const dagLinksToObjects = objectCids.map(objectCid => new dagPB.DAGLink('', 0, objectCid));
  const serializedData = JSON.stringify(packetData);

  return createDagNode(serializedData, dagLinksToObjects);
};
