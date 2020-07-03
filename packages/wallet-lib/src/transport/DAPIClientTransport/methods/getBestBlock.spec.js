const { expect } = require('chai');
const { Block } = require('@dashevo/dashcore-lib');

const DAPIClientTransport = require('../DAPIClientTransport');

describe('transports - DAPIClientTransport - .getBestBlock', function suite() {
  let bestBlockHash;
  let block;
  let transport;
  let clientMock;

  beforeEach(() => {
    bestBlockHash = '0000004bb65f29621dddcb85eb0d4aa3921e856097813b00d7784514809968ad';
    block = {
      header: {
        hash: '0000004bb65f29621dddcb85eb0d4aa3921e856097813b00d7784514809968ad', version: 536870912, prevHash: '000002243e872509388a6bd9c1c69c719bdcee2a780262f00c3cf75060f7adae', merkleRoot: '89724abcb2132645cffa8fdce002d9ced6d59e35231eaa0b3ddaf69f6c4e5c84', time: 1585673611, bits: 503479478, nonce: 24664,
      },
      transactions: [],
    };

    clientMock = {
      core: {
        getBestBlockHash: () => bestBlockHash,
        getBlockByHash: (hash) => {
          if (hash === bestBlockHash) return block;
          return null;
        },
      },
    }

    transport = new DAPIClientTransport(clientMock);
  })

  afterEach(() => {
    transport.disconnect();
  })

  it('should work', async () => {
    const res = await transport.getBestBlock();

    expect(res).to.deep.equal(new Block(block));
  });
});
