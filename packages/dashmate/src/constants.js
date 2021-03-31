const networks = {
  NETWORK_LOCAL: 'local',
  NETWORK_DEVNET: 'devnet',
  NETWORK_TESTNET: 'testnet',
  NETWORK_MAINNET: 'mainnet',
};

const presets = {
  PRESET_TESTNET: 'testnet',
  PRESET_LOCAL: 'local',
  PRESET_DEVNET: 'devnet',
  PRESET_MAINNET: 'mainnet',
};

const nodeTypes = {
  NODE_TYPE_MASTERNODE: 'masternode',
  NODE_TYPE_FULLNODE: 'fullnode',
};

const quorumTypes = {
  LLMQ_TYPE_TEST: 'llmq_test',
};

const MASTERNODE_DASH_AMOUNT = 1000;

module.exports = {
  ...networks,
  ...presets,
  ...nodeTypes,
  ...quorumTypes,
  NETWORKS: Object.values(networks),
  PRESETS: Object.values(presets),
  NODE_TYPES: Object.values(nodeTypes),
  QUORUM_TYPES: Object.values(quorumTypes),
  MASTERNODE_DASH_AMOUNT,
};
