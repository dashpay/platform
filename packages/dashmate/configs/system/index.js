const baseConfig = require('./base');
const localConfig = require('./local');
const testnetConfig = require('./testnet');

module.exports = {
  base: baseConfig,
  local: localConfig,
  testnet: testnetConfig,
};
