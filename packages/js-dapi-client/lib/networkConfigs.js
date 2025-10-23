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
    //  curl https://quorums.testnet.networks.dash.org/masternodes | jq 'select(.success) | .data | map(select(.status=="ENABLED" and .versionCheck=="success") | .address | sub(":[0-9]+$"; ":1443")) | sort_by( split(":")[0] | split(".") | map(tonumber) )' // eslint-disable-line max-len
    dapiAddressesWhiteList: [
      '34.214.48.68:1443',
      '35.82.197.197:1443',
      '35.85.21.179:1443',
      '35.163.144.230:1443',
      '35.164.23.245:1443',
      '35.167.145.149:1443',
      '44.227.137.77:1443',
      '44.228.242.181:1443',
      '44.239.39.153:1443',
      '44.240.98.102:1443',
      '52.10.229.11:1443',
      '52.12.176.90:1443',
      '52.13.132.146:1443',
      '52.24.124.162:1443',
      '52.33.28.47:1443',
      '52.34.144.50:1443',
      '52.43.13.92:1443',
      '52.43.86.231:1443',
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
