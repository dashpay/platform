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

// number of blocks to wait before core DKG exchange session
export const MIN_BLOCKS_BEFORE_DKG = 6;

export const PACKAGE_ROOT_DIR = path.join(url.fileURLToPath(import.meta.url), '../..');
export const TEMPLATES_DIR = path.join(PACKAGE_ROOT_DIR, 'templates');

const { version } = JSON.parse(fs.readFileSync(path.join(PACKAGE_ROOT_DIR, 'package.json'), 'utf8'));

export const DASHMATE_VERSION = version;
export const DASHMATE_HELPER_DOCKER_IMAGE = `dashpay/dashmate-helper:${version}`;

export const OUTPUT_FORMATS = {
  JSON: 'json',
  PLAIN: 'plain',
};

/**
 * ACME directory certificates are requested from.
 *
 * Let's Encrypt is the only public CA that issues certificates for IP
 * addresses over ACME, and Dashmate identifies a node by its external IP, so
 * this is the only value that obtains a publicly trusted certificate today.
 * It is configurable so an operator can rehearse against the staging directory
 * without spending the production failed-validation budget, and so a test can
 * point at a local ACME server.
 */
export const LETSENCRYPT_ACME_DIRECTORY_URL = 'https://acme-v02.api.letsencrypt.org/directory';

export const SSL_PROVIDERS = {
  ZEROSSL: 'zerossl',
  LETSENCRYPT: 'letsencrypt',
  FILE: 'file',
  SELF_SIGNED: 'self-signed',
};
