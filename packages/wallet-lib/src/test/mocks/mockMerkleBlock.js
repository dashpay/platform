const { MerkleBlock } = require('@dashevo/dashcore-lib');
const { genesis } = require('@dashevo/dash-spv');

const getRoot = (network) => {
  switch (network) {
    case 'testnet':
      return genesis.getTestnetGenesis();
    case 'devnet':
      return genesis.getDevnetGenesis();
    case 'regtest':
      return genesis.getRegtestGenesis();
    case 'livenet':
    case 'mainnet':
      return genesis.getLivenetGenesis();
    default:
      break;
  }

  return null;
};

const mockMerkleBlock = (txHashes, prevHeader, network = 'livenet') => {
  const header = prevHeader || getRoot(network);

  return new MerkleBlock({
    header,
    numTransactions: txHashes.length,
    hashes: txHashes
      .map((hash) => Buffer.from(hash, 'hex').reverse().toString('hex')),
    flags: [],
  });
};

module.exports = mockMerkleBlock;
