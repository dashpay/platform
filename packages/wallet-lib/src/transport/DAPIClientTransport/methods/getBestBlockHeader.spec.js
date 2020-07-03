const { expect } = require('chai');
const { Block } = require('@dashevo/dashcore-lib');

const DAPIClientTransport = require('../DAPIClientTransport');

describe('transports - DAPIClientTransport - .getBestBlockHeader', function suite() {
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
      transactions: [{
        hash: '89724abcb2132645cffa8fdce002d9ced6d59e35231eaa0b3ddaf69f6c4e5c84',
        version: 3,
        inputs: [{
          prevTxId: '0000000000000000000000000000000000000000000000000000000000000000', outputIndex: 4294967295, sequenceNumber: 4294967295, script: '028c300109',
        }],
        outputs: [{ satoshis: 6885000000, script: '76a91416b93a3b9168a20605cc3cda62f6135a3baa531a88ac' }, { satoshis: 6885000000, script: '76a91416b93a3b9168a20605cc3cda62f6135a3baa531a88ac' }],
        nLockTime: 0,
        type: 5,
        extraPayload: '02008c300000cead425668f38cfbb8dc028ad53d163fcee7282ede84d9a577ac6851a847ebc80000000000000000000000000000000000000000000000000000000000000000',
      }],
    };


    clientMock = {
      core: {
        getBestBlockHash: () => bestBlockHash,
        getBlockByHash: (hash) => {
          if (hash === bestBlockHash) return block;
          return null;
        },
      }
    }

    transport = new DAPIClientTransport(clientMock);
  })

  afterEach(() => {
    transport.disconnect();
  })

  it('should work', async () => {
    const res = await transport.getBestBlockHeader();

    expect(res).to.deep.equal(new Block(block).header);
  });
});
