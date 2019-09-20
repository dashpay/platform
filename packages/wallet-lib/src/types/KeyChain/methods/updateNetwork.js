const { Networks } = require('@dashevo/dashcore-lib');

function updateNetwork(network = JSON.parse(JSON.stringify(Networks.testnet.toString()))) {
  this.network = network;
}
module.exports = updateNetwork;
