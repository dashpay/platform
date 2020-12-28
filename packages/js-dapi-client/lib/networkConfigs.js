module.exports = {
  testnet: {
    seeds: [
      'seed-1.testnet.networks.dash.org',
      'seed-2.testnet.networks.dash.org',
      'seed-3.testnet.networks.dash.org',
      'seed-4.testnet.networks.dash.org',
      'seed-5.testnet.networks.dash.org',
    ],
    network: 'testnet',
  },
  evonet: {
    seeds: [
      'seed-1.evonet.networks.dash.org',
      'seed-2.evonet.networks.dash.org',
      'seed-3.evonet.networks.dash.org',
      'seed-4.evonet.networks.dash.org',
      'seed-5.evonet.networks.dash.org',
    ],
    network: 'evonet',
  },
  local: {
    dapiAddresses: ['127.0.0.1'],
    network: 'regtest',
  },
};
