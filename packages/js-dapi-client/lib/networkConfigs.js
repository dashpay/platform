module.exports = {
  testnet: {
    seeds: [
      'seed-1.testnet.networks.dash.org:1443',
      'seed-2.testnet.networks.dash.org:1443',
      'seed-3.testnet.networks.dash.org:1443',
      'seed-4.testnet.networks.dash.org:1443',
      'seed-5.testnet.networks.dash.org:1443',
      'seed-1.pshenmic.dev:1443',
    ],
    network: 'testnet',
    // Since we don't have PoSe atm, 3rd party masternodes sometimes provide wrong data
    // that breaks test suite and application logic. Temporary solution is to hardcode
    // reliable DCG testnet masternodes to connect. Should be removed when PoSe is introduced.
    // Generate list with:
    //  dash-cli -testnet masternodelist evo | jq 'to_entries | map(select(.value.status == "ENABLED") | .value.address | sub(":19999"; ":1443"))'
    dapiAddressesWhiteList: [
      '34.214.48.68:1443',
      '35.82.197.197:1443',
      '35.85.21.179:1443',
      '35.163.144.230:1443',
      '35.167.145.149:1443',
      '44.227.137.77:1443',
      '44.228.242.181:1443',
      '44.239.39.153:1443',
      '44.240.98.102:1443',
      '52.10.229.11:1443',
      '52.12.176.90:1443',
      '52.24.124.162:1443',
      '52.33.28.47:1443',
      '52.34.144.50:1443',
      '52.40.219.41:1443',
      '52.43.13.92:1443',
      '52.43.86.231:1443',
      '52.89.154.48:1443',
      '54.68.235.201:1443',
      '54.149.33.167:1443',
      '54.187.14.232:1443',
      '54.201.32.131:1443',
    ],
  },
  local: {
    dapiAddresses: ['127.0.0.1'],
    network: 'regtest',
  },
  mainnet: {
    seeds: [
      'seed-1.mainnet.networks.dash.org',
      'seed-2.mainnet.networks.dash.org',
      'seed-3.mainnet.networks.dash.org',
      'seed-4.mainnet.networks.dash.org',
      'seed-1.pshenmic.dev',
    ],
    network: 'mainnet',
  },
};
