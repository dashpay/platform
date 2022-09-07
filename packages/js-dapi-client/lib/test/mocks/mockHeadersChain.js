const X11 = require('wasm-x11-hash');
const { BlockHeader, configure } = require('@dashevo/dashcore-lib');
const { genesis } = require('@dashevo/dash-spv');

const getRoot = (network) => {
  switch (network) {
    case 'testnet':
      return genesis.getTestnetGenesis();
    case 'devnet':
      return genesis.getDevnetGenesis();
    case 'regtest':
      return genesis.getRegtestGenesis();
    default:
      break;
  }

  return null;
};

const BLOCK_TIME = 2.5 * 60;

let x11;

const mockHeadersChain = async (network, length, root) => {
  if (!x11) {
    x11 = await X11();
    // Configure Dashcore lib to operate with wasm x11
    configure({
      x11hash: x11,
    });
  }

  const rootHeader = root || getRoot(network);

  const chain = [rootHeader];

  let prevHeader = rootHeader;
  for (let i = 0; i < length - 1; i += 1) {
    const header = new BlockHeader({
      version: prevHeader.version,
      prevHash: Buffer.from(prevHeader.hash, 'hex').reverse(),
      merkleRoot: Buffer.alloc(32),
      time: prevHeader.time + BLOCK_TIME,
      bits: prevHeader.bits,
      nonce: 3861367235,
    });

    chain.push(header);
    prevHeader = header;
  }

  return chain;
};

module.exports = mockHeadersChain;
