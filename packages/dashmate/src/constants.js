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
};

const nodeTypes = {
  NODE_TYPE_MASTERNODE: 'masternode',
  NODE_TYPE_FULLNODE: 'fullnode',
};

module.exports = {
  ...networks,
  ...presets,
  ...nodeTypes,
  NETWORKS: Object.values(networks),
  PRESETS: Object.values(presets),
  NODE_TYPES: Object.values(nodeTypes),
};
