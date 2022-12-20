const X11 = require('wasm-x11-hash');
const { BlockHeader, MerkleBlock, configure } = require('@dashevo/dashcore-lib');
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

const BLOCK_TIME = 2.5 * 60;

const initX11 = async () => {
  const x11 = await X11();
  // Configure Dashcore lib to operate with wasm x11
  configure({
    x11hash: x11,
  });
};

initX11().catch(console.error);

/**
 * Mock block header
 * @param prevHeader
 * @param network
 * @return {BlockHeader}
 */
const mockHeader = (prevHeader, network = 'livenet') => {
  let prev = prevHeader;
  if (!prev) {
    prev = getRoot(network);
  }

  return new BlockHeader({
    version: prev.version,
    prevHash: Buffer.from(prev.hash, 'hex').reverse(),
    merkleRoot: Buffer.alloc(32),
    time: prev.time + BLOCK_TIME,
    bits: prev.bits,
    nonce: 3861367235,
  });
};

const mockHeadersChain = (network, length, root) => {
  const rootHeader = root || getRoot(network);

  const chain = [rootHeader];

  let prevHeader = rootHeader;
  for (let i = 0; i < length - 1; i += 1) {
    const header = mockHeader(prevHeader, network);

    chain.push(header);
    prevHeader = header;
  }

  return chain;
};

const mockMerkleBlock = (txHashes, prevHeader, network = 'livenet') => {
  const header = prevHeader ? mockHeader(prevHeader, network) : getRoot(network);

  return new MerkleBlock({
    header,
    numTransactions: txHashes.length,
    hashes: txHashes
      .map((hash) => Buffer.from(hash, 'hex').reverse().toString('hex')),
    flags: [],
  });
};

module.exports = {
  mockHeadersChain,
  mockHeader,
  mockMerkleBlock,
};
