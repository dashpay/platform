const path = require('path');

const networks = {
  NETWORK_LOCAL: 'local',
  NETWORK_DEVNET: 'devnet',
  NETWORK_TESTNET: 'testnet',
  NETWORK_MAINNET: 'mainnet',
};

const presets = {
  PRESET_MAINNET: 'mainnet',
  PRESET_TESTNET: 'testnet',
  PRESET_LOCAL: 'local',
};

const nodeTypes = {
  NODE_TYPE_MASTERNODE: 'masternode',
  NODE_TYPE_FULLNODE: 'fullnode',
};

const quorumNames = {
  LLMQ_TYPE_TEST: 'llmq_test',
};

const quorumTypes = {
  LLMQ_TYPE_TEST: 100,
};

const MASTERNODE_COLLATERAL_AMOUNT = 1000;
const HPMN_COLLATERAL_AMOUNT = 4000;

const PACKAGE_ROOT_DIR = path.join(__dirname, '..');
const TEMPLATES_DIR = path.join(PACKAGE_ROOT_DIR, 'templates');

const OUTPUT_FORMATS = {
  JSON: 'json',
  PLAIN: 'plain',
};

const SSL_PROVIDERS = {
  ZEROSSL: 'zerossl',
  FILE: 'file',
  SELF_SIGNED: 'self-signed',
};

module.exports = {
  ...networks,
  ...presets,
  ...nodeTypes,
  ...quorumNames,
  NETWORKS: Object.values(networks),
  PRESETS: Object.values(presets),
  NODE_TYPES: Object.values(nodeTypes),
  QUORUM_NAMES: Object.values(quorumNames),
  QUORUM_TYPES: quorumTypes,
  MASTERNODE_COLLATERAL_AMOUNT,
  HPMN_COLLATERAL_AMOUNT,
  PACKAGE_ROOT_DIR,
  TEMPLATES_DIR,
  OUTPUT_FORMATS,
  SSL_PROVIDERS,
  SSL_PROVIDERS_LIST: Object.values(SSL_PROVIDERS),
};
