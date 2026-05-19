import X11 from 'wasm-x11-hash';
import dashcore from '@dashevo/dashcore-lib';
const { BlockHeader, configure } = dashcore;
import { genesis } from '@dashevo/dash-spv';
import { hexToBytes } from '../../utils/bytes.js';

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
      prevHash: hexToBytes(prevHeader.hash).reverse(),
      merkleRoot: new Uint8Array(32),
      time: prevHeader.time + BLOCK_TIME,
      bits: prevHeader.bits,
      nonce: 3861367235,
    });

    chain.push(header);
    prevHeader = header;
  }

  return chain;
};

export default mockHeadersChain;
