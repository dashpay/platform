import path from 'path';
import fs from 'fs';
import url from 'url';

export const NETWORK_LOCAL = 'local';
export const NETWORK_DEVNET = 'devnet';
export const NETWORK_TESTNET = 'testnet';
export const NETWORK_MAINNET = 'mainnet';

export const NETWORKS = {
  NETWORK_LOCAL,
  NETWORK_DEVNET,
  NETWORK_TESTNET,
  NETWORK_MAINNET,
};

export const PRESET_MAINNET = 'mainnet';
export const PRESET_TESTNET = 'testnet';
export const PRESET_LOCAL = 'local';

export const PRESETS = {
  PRESET_MAINNET,
  PRESET_TESTNET,
  PRESET_LOCAL,
};

export const NODE_TYPE_MASTERNODE = 'masternode';
export const NODE_TYPE_FULLNODE = 'fullnode';

export const LLMQ_TYPE_TEST = 'llmq_test';
export const LLMQ_TYPE_TEST_PLATFORM = 'llmq_test_platform';

export const QUORUM_TYPES = {
  LLMQ_TYPE_TEST: 100,
  LLMQ_TYPE_TEST_PLATFORM: 106,
};

export const MASTERNODE_COLLATERAL_AMOUNT = 1000;
export const HPMN_COLLATERAL_AMOUNT = 4000;

export const PACKAGE_ROOT_DIR = path.join(url.fileURLToPath(import.meta.url), '../..');
export const TEMPLATES_DIR = path.join(PACKAGE_ROOT_DIR, 'templates');

const { version } = JSON.parse(fs.readFileSync(path.join(PACKAGE_ROOT_DIR, 'package.json'), 'utf8'));

export const DASHMATE_HELPER_DOCKER_IMAGE = `dashpay/dashmate-helper:${version}`;

export const OUTPUT_FORMATS = {
  JSON: 'json',
  PLAIN: 'plain',
};

export const SSL_PROVIDERS = {
  ZEROSSL: 'zerossl',
  FILE: 'file',
  SELF_SIGNED: 'self-signed',
};
