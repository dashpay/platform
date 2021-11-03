const baseConfig = require('./base');
const localConfig = require('./local');
const testnetConfig = require('./testnet');
const mainnetConfig = require('./mainnet');

module.exports = {
  base: baseConfig,
  local: localConfig,
  testnet: testnetConfig,
  mainnet: mainnetConfig,
};
