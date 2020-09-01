const _ = require('lodash');
const { expect } = require('chai');
const { HDPrivateKey, Transaction } = require('@dashevo/dashcore-lib');

const createTransaction = require('./createTransaction');
const { mnemonic } = require('../../../../fixtures/wallets/mnemonics/during-develop-before');
const FixtureTransport = require('../../../transport/FixtureTransport/FixtureTransport');

const getUTXOS = require('./getUTXOS');
const { simpleDescendingAccumulator } = require('../../../utils/coinSelections/strategies');

const addressesFixtures = require('../../../../fixtures/addresses.json');
const fixtureUTXOS = require('../../../transport/FixtureTransport/data/utxos/yQ1fb64aeLfgqFKyeV9Hg9KTaTq5ehHm22.json');
const validStore = require('../../../../fixtures/walletStore').valid.orange.store;

const craftedGenerousMinerStrategy = require('../../../../fixtures/strategies/craftedGenerousMinerStrategy');

describe('Account - createTransaction', function suite() {
  this.timeout(10000);
  let mockWallet;

  it('sould warn on missing inputs', function () {
    const self = {
      store: validStore,
      walletId: 'a3771aaf93',
      getUTXOS,
      network: 'testnet'
    };

    const mockOpts1 = {};
    const mockOpts2 = {
      satoshis: 1000,
    };
    const mockOpts3 = {
      satoshis: 1000,
      recipient: addressesFixtures.testnet.valid.yereyozxENB9jbhqpbg1coE5c39ExqLSaG.addr,
    };
    const expectedException1 = 'An amount in dash or in satoshis is expected to create a transaction';
    const expectedException2 = 'A recipient is expected to create a transaction';
    const expectedException3 = 'Error: utxosList must contain at least 1 utxo';
    expect(() => createTransaction.call(self, mockOpts1)).to.throw(expectedException1);
    expect(() => createTransaction.call(self, mockOpts2)).to.throw(expectedException2);
    expect(() => createTransaction.call(self, mockOpts3)).to.throw(expectedException3);
  });

  it('should create valid and deterministic transactions', async function () {
    if(process.browser){
      // FixtureTransport relies heavily on fs.existSync and fs.readFile which are not available on browser
      this.skip('FixtureTransport do not support browser environment due to FS intensive usage');
      return;
    }
    const transport = new FixtureTransport();
    transport.setHeight(21546);

    mockWallet = {
      getUTXOS: () => fixtureUTXOS["21546"].map(utxo => Transaction.UnspentOutput(utxo)),
      getUnusedAddress: () => {
        return {"address": 'yMGXHsi8gstbd5wqfqkqcfsbwJjGBt5sWu'}
      },
      getPrivateKeys: (addrList) => {
        if (addrList.length === 1 && addrList[0] === 'yQ1fb64aeLfgqFKyeV9Hg9KTaTq5ehHm22') {
          return [new HDPrivateKey('tprv8jG3ctd1DEVADnLP3hwS1Gfzjxf5E4WL2UutfJkhAQs7rVu2b3Ryv4WQ46mddZyMbGaSUYnY9wFeuFRAejapjoB1LGzTfM55mxMhZ1X4eGX')]
        }
      },
      keyChain: {
        sign: (tx, privateKeys) => tx.sign(privateKeys),
      },
      storage: {
        searchTransaction: (txId) => {
          const tx = transport.getTransaction(txId);
          if (tx) {
            return {found: true, result: tx, hash: txId}
          } else {
            return {found: false, hash: txId}
          }
        }
      }
    };
    const expectedTx1 = '0300000001b64e23b6bd8c1016c8595ab6256e97ac5a33a95b5c68cc99410bf88867023910000000006a47304402200f8851bfcba02f1375c9d14cc1e4a1f442a6ba04dade5060124b6d245738eb1502206f2655f5e3714e9a1aa46de58124ec44d4da36884db0f1a39e6cad912ce009fc012103987110fc08c848657176385b37a77fb7f6d89bc873bb4334146ffe44ac126566ffffffff0250c30000000000001976a9140a6a961f1c664a9cd004c593381dd4d9f1f5463588acb9059a3b000000001976a9140a6a961f1c664a9cd004c593381dd4d9f1f5463588ac00000000';
    const expectedTx2 = '0300000001b64e23b6bd8c1016c8595ab6256e97ac5a33a95b5c68cc99410bf88867023910000000006b483045022100fc88e4585654961610e375b19f33b52d10e1c7efa5ef91531c627129538cf7ef0220108a281374a691522b5deb51ce3249723efe9541e57a4de87bdd8ba7ce43ce8e012103987110fc08c848657176385b37a77fb7f6d89bc873bb4334146ffe44ac126566ffffffff0b804a5d05000000001976a9140a6a961f1c664a9cd004c593381dd4d9f1f5463588ac804a5d05000000001976a91403ab1053a3bc741a012607893c66565c6815b9d888ac804a5d05000000001976a9146c773e3b74a16931f995288645f4f6379076048688ac804a5d05000000001976a914429dfc6b9a9d86463ea65b55d8cedb26a5e04f3388ac804a5d05000000001976a91434cb4bfb6e27ed0067e47c55da615bf7230e23f888ac804a5d05000000001976a914eb9a36fab9220e5e966fdcfe1abf2ee43308cb5d88ac804a5d05000000001976a9144c9f7ef1c5af5f0d2b219a035a46c7f54035b0a288ac804a5d05000000001976a9141c44d8966f001ddb7cea277edc33b02f151b603788ac804a5d05000000001976a914f4159f063a076038a484cf9d027808dbac118a1a88ac804a5d05000000001976a9147bc630538f5bb87d3166b6cf5f69853809235f4388acdcdef505000000001976a9140a6a961f1c664a9cd004c593381dd4d9f1f5463588ac00000000';

    const tx1 = await createTransaction.call(mockWallet, {
      recipient: 'yMGXHsi8gstbd5wqfqkqcfsbwJjGBt5sWu',
      satoshis: 50000,
    });
    expect(tx1.toString('hex')).to.deep.equal(expectedTx1);

    const tx2 = await createTransaction.call(mockWallet, {
      recipients: [
        {
          recipient: 'yMGXHsi8gstbd5wqfqkqcfsbwJjGBt5sWu',
          satoshis: 90000000
        }, {
          recipient: 'yLeqoVqqGf4hFDwsiJwKiLPpeJbZHJpwo7',
          satoshis: 90000000
        }, {
          recipient: 'yWCxg5NdRXDagFokjwdLMYNDqfEKmLPtua',
          satoshis: 90000000
        }, {
          recipient: 'ySPghvb9M1PqjhRYKv7iivQEuebM2aXs9f',
          satoshis: 90000000
        }, {
          recipient: 'yR8bXVFZAM1ysc8s4GfVTirNhTEzKizY19',
          satoshis: 90000000
        }, {
          recipient: 'yhoCPK6WyqtB5GmZjVqxy3faR5JMUKbt8x',
          satoshis: 90000000
        }, {
          recipient: 'yTJbGkT7TYVY4MYbTgdSDdq19A3VmjyEUo',
          satoshis: 90000000
        }, {
          recipient: 'yNtvF5g6qnbRsUJ8ggap3pd53HEmkngEJu',
          satoshis: 90000000
        }, {
          recipient: 'yia3dGyRdh7xZLDtum1rdCLRqabyBQbcWL',
          satoshis: 90000000
        }, {
          recipient: 'yXbuPCJagq4XH85hgxqsNv92kSUFroTWUA',
          satoshis: 90000000
        },
      ]
    });
    expect(tx2.toString('hex')).to.equal(expectedTx2);
  });
  it('should be able to create transaction with specific strategy', async function () {
    if(process.browser){
      // FixtureTransport relies heavily on fs.existSync and fs.readFile which are not available on browser
      this.skip('FixtureTransport do not support browser environment due to FS intensive usage');
      return;
    }
    const expectedTxStd = '0300000001b64e23b6bd8c1016c8595ab6256e97ac5a33a95b5c68cc99410bf88867023910000000006a47304402200f8851bfcba02f1375c9d14cc1e4a1f442a6ba04dade5060124b6d245738eb1502206f2655f5e3714e9a1aa46de58124ec44d4da36884db0f1a39e6cad912ce009fc012103987110fc08c848657176385b37a77fb7f6d89bc873bb4334146ffe44ac126566ffffffff0250c30000000000001976a9140a6a961f1c664a9cd004c593381dd4d9f1f5463588acb9059a3b000000001976a9140a6a961f1c664a9cd004c593381dd4d9f1f5463588ac00000000';
    const expectedTxStrat = '0300000001b64e23b6bd8c1016c8595ab6256e97ac5a33a95b5c68cc99410bf88867023910000000006a4730440220171da851d2915f7faa20a7d7aa66383c93cca6b623d12cdb1919d913abe558aa0220154f7edac296e3e2cd393e46f18baf9f4463aaa0a6d2ce5259055280ed05d878012103987110fc08c848657176385b37a77fb7f6d89bc873bb4334146ffe44ac126566ffffffff0250c30000000000001976a9140a6a961f1c664a9cd004c593381dd4d9f1f5463588acad059a3b000000001976a9140a6a961f1c664a9cd004c593381dd4d9f1f5463588ac00000000';

    const txStdStrategy = await createTransaction.call(mockWallet, {
      recipient: 'yMGXHsi8gstbd5wqfqkqcfsbwJjGBt5sWu',
      satoshis: 50000,
    });
    expect(txStdStrategy.toString('hex')).to.deep.equal(expectedTxStd);

    mockWallet.strategy = craftedGenerousMinerStrategy
    const txStrat1 = await createTransaction.call(mockWallet, {
      recipient: 'yMGXHsi8gstbd5wqfqkqcfsbwJjGBt5sWu',
      satoshis: 50000,
    });
    expect(txStrat1.toString('hex')).to.deep.equal(expectedTxStrat);
    const txStrat2 = await createTransaction.call(mockWallet, {
      recipient: 'yMGXHsi8gstbd5wqfqkqcfsbwJjGBt5sWu',
      satoshis: 50000,
      strategy: craftedGenerousMinerStrategy
    });
    expect(txStrat2.toString('hex')).to.deep.equal(expectedTxStrat);
  });
});
