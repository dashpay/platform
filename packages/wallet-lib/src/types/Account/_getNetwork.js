const Dashcore = require('@dashevo/dashcore-lib');

module.exports = function getNetwork(network) {
  return Dashcore.Networks[network].toString() || Dashcore.Networks.testnet.toString();
};
