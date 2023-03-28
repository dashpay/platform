const {
  NODE_TYPE_MASTERNODE,
  NODE_TYPE_FULLNODE,
} = require('../../../constants');

const NODE_TYPE_NAMES = {
  MASTERNODE: 'masternode',
  HP_MASTERNODE: 'high-performance masternode',
  FULLNODE: 'fullnode',
  HP_FULLNODE: 'high-performance fullnode',
};

const NODE_TYPE_NAME_TYPES = {
  [NODE_TYPE_NAMES.MASTERNODE]: NODE_TYPE_MASTERNODE,
  [NODE_TYPE_NAMES.HP_MASTERNODE]: NODE_TYPE_MASTERNODE,
  [NODE_TYPE_NAMES.FULLNODE]: NODE_TYPE_FULLNODE,
  [NODE_TYPE_NAMES.HP_FULLNODE]: NODE_TYPE_FULLNODE,
};

function isNodeTypeNameHighPerformance(nodeTypeName) {
  return [NODE_TYPE_NAMES.HP_MASTERNODE, NODE_TYPE_NAMES.HP_FULLNODE].includes(nodeTypeName);
}

function getNodeTypeByName(nodeTypeName) {
  return NODE_TYPE_NAME_TYPES[nodeTypeName];
}

module.exports = {
  NODE_TYPE_NAMES,
  isNodeTypeNameHighPerformance,
  getNodeTypeByName,
};
