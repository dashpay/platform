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
const MIN_DIFFICULTY_BLOCK_TIME = (2 * 60 * 60) + 1;

let x11;

const mockHeadersChain = async (network, length, root, options = {}) => {
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
    let nonce = options.mine ? 0 : 3861367235;
    let header;
    do {
      header = new BlockHeader({
        version: prevHeader.version,
        prevHash: Buffer.from(prevHeader.hash, 'hex').reverse(),
        merkleRoot: Buffer.alloc(32),
        // Regtest headers spaced by more than two hours use the canonical
        // minimum-difficulty target, keeping long mined fixtures cheap while
        // still exercising the full consensus validator.
        time: prevHeader.time + (options.mine ? MIN_DIFFICULTY_BLOCK_TIME : BLOCK_TIME),
        bits: prevHeader.bits,
        nonce,
      });
      nonce += 1;
    } while (options.mine && !header.validProofOfWork());

    chain.push(header);
    prevHeader = header;
  }

  return chain;
};

module.exports = mockHeadersChain;
