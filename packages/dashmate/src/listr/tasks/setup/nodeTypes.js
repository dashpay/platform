import {
  NODE_TYPE_MASTERNODE,
  NODE_TYPE_FULLNODE,
} from '../../../constants';

const NODE_TYPE_NAMES = {
  MASTERNODE: 'masternode',
  HP_MASTERNODE: 'evolution masternode',
  FULLNODE: 'fullnode',
  HP_FULLNODE: 'evolution fullnode',
};

const NODE_TYPE_NAME_TYPES = {
  [NODE_TYPE_NAMES.MASTERNODE]: NODE_TYPE_MASTERNODE,
  [NODE_TYPE_NAMES.HP_MASTERNODE]: NODE_TYPE_MASTERNODE,
  [NODE_TYPE_NAMES.FULLNODE]: NODE_TYPE_FULLNODE,
  [NODE_TYPE_NAMES.HP_FULLNODE]: NODE_TYPE_FULLNODE,
};

const NODE_TYPE_NAME_BY_TYPE = {
  [NODE_TYPE_MASTERNODE]: NODE_TYPE_NAMES.MASTERNODE,
  [NODE_TYPE_FULLNODE]: NODE_TYPE_NAMES.FULLNODE,
};

function isNodeTypeNameHighPerformance(nodeTypeName) {
  return [NODE_TYPE_NAMES.HP_MASTERNODE, NODE_TYPE_NAMES.HP_FULLNODE].includes(nodeTypeName);
}

function getNodeTypeByName(nodeTypeName) {
  return NODE_TYPE_NAME_TYPES[nodeTypeName];
}

function getNodeTypeNameByType(nodeType) {
  return NODE_TYPE_NAME_BY_TYPE[nodeType];
}

export default {
  NODE_TYPE_NAMES,
  isNodeTypeNameHighPerformance,
  getNodeTypeByName,
  getNodeTypeNameByType,
};
