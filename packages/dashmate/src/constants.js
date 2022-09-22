const path = require('path');
const os = require('os');

const networks = {
  NETWORK_LOCAL: 'local',
  NETWORK_DEVNET: 'devnet',
  NETWORK_TESTNET: 'testnet',
  NETWORK_MAINNET: 'mainnet',
};

const presets = {
  PRESET_LOCAL: 'local',
  PRESET_TESTNET: 'testnet',
  PRESET_MAINNET: 'mainnet',
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

const MASTERNODE_DASH_AMOUNT = 1000;

const HOME_DIR_PATH = process.env.DASHMATE_HOME_DIR
  ? process.env.DASHMATE_HOME_DIR
  : path.resolve(os.homedir(), '.dashmate');
const CONFIG_FILE_PATH = path.join(HOME_DIR_PATH, 'config.json');

const OUTPUT_FORMATS = {
  JSON: 'json',
  PLAIN: 'plain',
};

const sslProviders = {
  ZEROSSL: 'zerossl',
  MANUL: 'manual',
  SELF_SIGNED: 'selfSigned',
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
  MASTERNODE_DASH_AMOUNT,
  HOME_DIR_PATH,
  CONFIG_FILE_PATH,
  OUTPUT_FORMATS,
  SSL_PROVIDERS: Object.values(sslProviders),
};
