const { expect } = require('chai');
const Dashcore = require('@dashevo/dashcore-lib');
const Wallet = require('../../../src/Wallet');
const { mnemonicString1 } = require('../../fixtures.json');
const InsightClient = require('../../../src/transports/Insight/insightClient');

let wallet = null;
let account = null;
const insightClientOpts = {
  uri: 'https://testnet-insight.dashevo.org/insight-api-dash',
  socketUri: 'https://testnet-insight.dashevo.org/',
  useSocket: false,
};

describe('Transport : Insight Client', function suite() {
  this.timeout(20000);
  before((done) => {
    const insight = new InsightClient(insightClientOpts);
    const config = {
      transport: insight,
      mnemonic: mnemonicString1,
      network: Dashcore.Networks.testnet,
    };

    wallet = new Wallet(config);
    account = wallet.createAccount();
    console.log('Account created');
    account.events.on('ready', () => {
      done();
    });
  });
  it('should be able to setNetwork', () => {
    expect(account.updateNetwork('livenet')).to.equal(true);
    expect(account.network).to.equal(Dashcore.Networks.livenet);
    expect(account.updateNetwork('testnet')).to.equal(true);
    expect(account.network).to.equal(Dashcore.Networks.testnet);
  });
  it('should be able to subscribe to an event', () => {
    account.transport.transport.subscribeToEvent('noevent');
    expect(account.transport.transport.listeners.noevent).to.exist;
  });
  it('should be able to unsubscribe of an event', () => {
    account.transport.transport.unsubscribeFromEvent('noevent');
    expect(account.transport.transport.listeners.noevent).to.not.exist;
  });
  it('should subscribe to address', () => {
    account.transport.transport.subscribeToAddresses(['yiFNYQxfkDxeCBLyWtWQ9w8UGwYugERLCq']);
    expect(true).to.equal(true);
  });
  it('should be able to pass Insight Client as a transport layer', () => {
    expect(wallet.transport).to.not.equal(null);
    expect(wallet.transport.type).to.equal('InsightClient');
    expect(account.transport).to.not.equal(null);
    expect(account.transport.type).to.equal('InsightClient');
  });
  it('should be able to get the address information', () => {
    const addressesExternalData = account.getAddresses();
    const path = 'm/44\'/1\'/0\'';
    expect(addressesExternalData).to.have.property(`${path}/0/0`);
    expect(addressesExternalData[`${path}/0/0`]).to.have.property('transactions');
    const expectedTransactionsArr = [
      'e66474bfe8ae3d91b2784864fc09e0bd615cbfbf4a2164e46b970bcc488a938f',
      'b4f567f398ec2174df2d775c9bcbc197efda2902bc4b628858d6c8ef7453284d',
    ];
    expect(addressesExternalData[`${path}/0/0`].address).to.equal('yRdxQQpXYh9Xkd91peJ7FJxpEzoRb6droH');
    expect(addressesExternalData[`${path}/0/0`].transactions).to.deep.equal(expectedTransactionsArr);
    expect(addressesExternalData[`${path}/0/0`].utxos).to.deep.equal([]);
    expect(addressesExternalData[`${path}/0/0`].fetchedLast).to.be.greaterThan(1533535093789);
    expect(addressesExternalData[`${path}/0/0`].used).to.equal(true);
  });
  it('should be able to get the utxos information', () => {
    const addressesExternalData = account.getAddresses();
    const path = 'm/44\'/1\'/0\'/0/4';
    expect(addressesExternalData).to.have.property(path);
    expect(addressesExternalData[path]).to.have.property('utxos');
    expect(addressesExternalData[path].utxos).to.have.length(1);
    expect(addressesExternalData[path].utxos[0].outputIndex).to.equal(0);
    expect(addressesExternalData[path].utxos[0].address).to.equal('yiFNYQxfkDxeCBLyWtWQ9w8UGwYugERLCq');
    expect(addressesExternalData[path].utxos[0].satoshis).to.equal(5000000000);
    expect(addressesExternalData[path].utxos[0].script).to.equal('76a914f08d82224ffc020f3d7110e57c3105a5caec058f88ac');
    expect(addressesExternalData[path].utxos[0].txId).to.equal('4ae8d1960c9a4ed83dbeaf1ad94b4a82f11c8574207144beda87113d94a31da1');
    expect(addressesExternalData[path].balanceSat).to.equal(5000000000);
  });
  it('should be able to get the total balance of an account', () => {
    const balance = account.getBalance();
    const expectedBalance = 14219900000;
    expect(balance).to.equal(expectedBalance);
  });
  it('should be able to get a valid UTXO', () => {
    const expectedUTXOS = [
      {
        address: 'yiFNYQxfkDxeCBLyWtWQ9w8UGwYugERLCq',
        outputIndex: 0,
        satoshis: 5000000000,
        script: '76a914f08d82224ffc020f3d7110e57c3105a5caec058f88ac',
        txId: '4ae8d1960c9a4ed83dbeaf1ad94b4a82f11c8574207144beda87113d94a31da1',
      },
      {
        address: 'yfXQM8TaFiXFYtiFCSm3y6fRq15cj59vVK',
        outputIndex: 1,
        satoshis: 3999990000,
        script: '76a914d2ad4b21655c7e019077cdf759cd4c2a0b6682e988ac',
        txId: '6770dee69437c6bf83a56956a04b807ef78cc62b79369b9551f935a922acbf64',
      },
      {
        address: 'yN3RXNVbRxA2S4gyHweT9TbFKenuGKd7fW',
        outputIndex: 0,
        satoshis: 3899970000,
        script: '76a91412e87d8a188ff29049b6a9e871018de65aa079c288ac',
        txId: 'd928aedc4ecc6c251cabee0672c19308573e5b4898c32779f3fd211dd8a1fbd8',
      },
      {
        address: 'yMi854XzeEmAz9UczDCj9tvXeddweKc9JM',
        outputIndex: 1,
        satoshis: 899990000,
        script: '76a9140f42047f86d356426458eba372031f524af548ce88ac',
        txId: '6c42619dd84a02577458ba4f880fe8cfaced9ed518ee7c360c5b107d6ff5b62d',
      },
      {
        address: 'ySypFbLpFTXrBbpFqRezwpdwwuaDCfrgpo',
        outputIndex: 1,
        satoshis: 419950000,
        script: '76a91449126d84886a9bfc4a2a49aa5ba9cb45c994875288ac',
        txId: 'b42c5052d7d31a422e711d50d3754217b0b16b6dfa29cf497b3dd75afa4febcb',
      },
    ];

    const UTXOS = account.getUTXOS();
    expect(UTXOS).to.deep.equal(expectedUTXOS);
  });
  it('should be able to get an unused address', () => {
    const unusedExternal = account.getUnusedAddress();
    const unusedInternal = account.getUnusedAddress(false);
    expect(unusedExternal.address).to.equal('yf3KLBh1y5ZbNrcab8xr7DN7HPBGhSWoDY');
    expect(unusedInternal.address).to.equal('yWbRpgDDsAVjXVWGHqZgiEoP7Mop9hAwYS');
  });

  it('should be able to create a transaction', () => {
    const { address } = account.getUnusedAddress();

    expect(address).to.equal('yf3KLBh1y5ZbNrcab8xr7DN7HPBGhSWoDY');

    const txOpts = { amount: 15, to: address };
    const txOptsSatoshis = { satoshis: 1500000000, to: address };

    const expectedRawTx = '0300000001d8fba1d81d21fdf37927c398485b3e570893c17206eeab1c256ccc4edcae28d9000000006b483045022100d8436eeb4cbd6dbe7ea88f565eca606a06ba043a290e1bb84f0b234aabc8c3bb022037a014ff30b899c8e896c2ccb5d8b6a106bf56c4a6c0093ecd6ee786a50d984d01210389143a6bee1de5a9583c697e160655c703aa6f7199c93bf14287879a20cd01fbffffffff02002f6859000000001976a914cd5d758c0898175abfbcd12b13ebe783e1dc2b2b88acc07b0c8f000000001976a914d9e7ddb55be9df9d5678fda4845e9e1181750b9088ac00000000';
    const rawTxFromAmount = account.createTransaction(txOpts);
    const rawTxFromSatoshisAmount = account.createTransaction(txOptsSatoshis);
    expect(rawTxFromAmount).to.equal(expectedRawTx);
    expect(rawTxFromSatoshisAmount).to.equal(expectedRawTx);
  });
  it('should be able to create an instantSend transactions', () => {
    const { address } = account.getUnusedAddress();
    const txOptsInstant = {
      amount: 10,
      to: address,
      isInstantSend: true,
    };
    const expectedRawTx = '0300000001d8fba1d81d21fdf37927c398485b3e570893c17206eeab1c256ccc4edcae28d9000000006a473044022014a6d5b9e6c09edb4040baa8de6b8b21fe7d4f700d2395e4b165040a41c81b930220486ee8ec9c07aaebb50707ce72773927d3095c09369a686c999db587abdb5e5d01210389143a6bee1de5a9583c697e160655c703aa6f7199c93bf14287879a20cd01fbffffffff0200ca9a3b000000001976a914cd5d758c0898175abfbcd12b13ebe783e1dc2b2b88acc0e0d9ac000000001976a914d9e7ddb55be9df9d5678fda4845e9e1181750b9088ac00000000';
    const rawTx = account.createTransaction(txOptsInstant);
    expect(rawTx).to.equal(expectedRawTx);
  });

  it('should not be able to create an instantSend transactions without opts', () => {
    expect(() => account.createTransaction()).to.throw('An amount in dash or in satoshis is expected to create a transaction');
  });
  it('should not be able to create an instantSend transactions without amount', () => {
    const { address } = account.getUnusedAddress();
    const txOptsInstant = {
      to: address,
      isInstantSend: true,
    };
    expect(() => account.createTransaction(txOptsInstant)).to.throw('An amount in dash or in satoshis is expected to create a transaction');
  });
  it('should not be able to create an instantSend transactions without to', () => {
    const txOptsInstant = {
      amount: 10,
      isInstantSend: true,
    };
    expect(() => account.createTransaction(txOptsInstant)).to.throw('A recipient is expected to create a transaction');
  });

  it('should be able to create an instantSend transactions with satoshis', () => {
    const { address } = account.getUnusedAddress();
    const txOptsInstant = {
      satoshis: 11,
      to: address,
      isInstantSend: true,
    };
    const expectedRawTx = '0300000001a11da3943d1187dabe44712074851cf1824a4bd91aafbe3dd84e9a0c96d1e84a000000006a4730440220462e82c236a13d7800a5ed999a3347e4e8fd2d8f1fa91192491552ae2b4f661702202f3e880ccabb2fbf2618fae91c4a49127e2155d1b63be7a1f9123a7ccfff97a60121039c2ac9fcf618c9bbf3c358b9e391d2c6c0829cc740ab1d11621c369083d26078ffffffff020b000000000000001976a914cd5d758c0898175abfbcd12b13ebe783e1dc2b2b88ace5ca052a010000001976a914d9e7ddb55be9df9d5678fda4845e9e1181750b9088ac00000000';
    const rawTx = account.createTransaction(txOptsInstant);
    expect(rawTx).to.equal(expectedRawTx);
  });

  it('should be able to create an instantSend transactions with satoshis and amount. Ammount is ignored?', () => {
    const { address } = account.getUnusedAddress();
    const txOptsInstant = {
      amount: 10,
      satoshis: 11,
      to: address,
      isInstantSend: true,
    };
    const expectedRawTx = '0300000001a11da3943d1187dabe44712074851cf1824a4bd91aafbe3dd84e9a0c96d1e84a000000006a4730440220462e82c236a13d7800a5ed999a3347e4e8fd2d8f1fa91192491552ae2b4f661702202f3e880ccabb2fbf2618fae91c4a49127e2155d1b63be7a1f9123a7ccfff97a60121039c2ac9fcf618c9bbf3c358b9e391d2c6c0829cc740ab1d11621c369083d26078ffffffff020b000000000000001976a914cd5d758c0898175abfbcd12b13ebe783e1dc2b2b88ace5ca052a010000001976a914d9e7ddb55be9df9d5678fda4845e9e1181750b9088ac00000000';
    const rawTx = account.createTransaction(txOptsInstant);
    expect(rawTx).to.equal(expectedRawTx);
  });

  it('should be able to create a wallet without transport', () => {
    const wallet2 = new Wallet({
      mnemonic: 'wisdom claim quote stadium input danger planet angry crucial cargo struggle medal',
      network: 'testnet',
    });
    expect(wallet2.transport).to.not.equal(null);
    const acc1 = wallet2.createAccount({ mode: 'light' });
    const acc2 = wallet2.createAccount({ mode: 'light' });
    const acc3 = wallet2.createAccount({ mode: 'light' });


    [acc1, acc2, acc3].forEach((el, i) => {
      // eslint-disable-next-line no-unused-expressions
      expect(el).to.exist;
      expect(el).to.be.a('object');
      expect(el.constructor.name).to.equal('Account');
      expect(el.BIP44PATH).to.equal(`m/44'/1'/${i}'`);
    });
    wallet2.disconnect();
  });

  it('should be able to create a wallet and account with invalid transport', () => {
    const wallet3 = new Wallet({
      mnemonic: 'wisdom claim quote stadium input danger planet angry crucial cargo struggle medal',
      network: 'testnet',
      transport: 'haha',

    });
    expect(wallet3.transport).to.not.equal(null);
    expect(wallet3.transport.valid).to.equal(false);
    expect(wallet3.transport.type).to.equal('String');

    const acc1 = wallet3.createAccount({ mode: 'light' });
    const acc2 = wallet3.createAccount({ mode: 'light' });
    const acc3 = wallet3.createAccount({ mode: 'light' });
    [acc1, acc2, acc3].forEach((el, i) => {
      // eslint-disable-next-line no-unused-expressions
      expect(el).to.exist;
      expect(el).to.be.a('object');
      expect(el.constructor.name).to.equal('Account');
      expect(el.BIP44PATH).to.equal(`m/44'/1'/${i}'`);
    });
    expect(acc1.transport).to.not.equal(null);
    expect(acc1.transport).to.be.a('Object');
    expect(acc1.transport.valid).to.equal(false);
    expect(acc1.transport.type).to.equal('String');
    wallet3.disconnect();
  });

  it('should not be able to getAddressSummary by invalid value', () => {
    const transport = new InsightClient(insightClientOpts);
    return transport.getAddressSummary('address').then(
      () => Promise.reject(new Error('Expected method to reject.')),
      err => expect(err).to.be.a('Error').with.property('message', 'Request failed with status code 400'),
    ).then(() => { transport.closeSocket(); });
  });
  it('should get a transactions History', () => {
    const expected = [
      {
        from: [
          'yRdxQQpXYh9Xkd91peJ7FJxpEzoRb6droH',
        ],
        time: 1529233103,
        to: {
          address: 'yf6qYQzQoCzpF7gJYAa7s3n5rBK89RoaCQ',
          amount: '50.00000000',
        },
        txid: 'e66474bfe8ae3d91b2784864fc09e0bd615cbfbf4a2164e46b970bcc488a938f',
        type: 'sent',
      },
      {
        from: [
          'yfPzgAZasiJGbiaYfJq7zXNN58PJAhbV1R',
        ],
        time: 1529201724,
        to: {
          address: 'yRdxQQpXYh9Xkd91peJ7FJxpEzoRb6droH',
          amount: '100.00000000',
        },
        txid: 'b4f567f398ec2174df2d775c9bcbc197efda2902bc4b628858d6c8ef7453284d',
        type: 'receive',
      },
      {
        from: [
          'yf6qYQzQoCzpF7gJYAa7s3n5rBK89RoaCQ',
        ],
        time: 1533535851,
        to: {
          address: 'yRwh2qqnSgWKSaE7Vob35JY4wprvx8ujPZ',
          amount: '10.00000000',
        },
        txid: '6770dee69437c6bf83a56956a04b807ef78cc62b79369b9551f935a922acbf64',
        type: 'sent',
      },
      {
        from: [
          'yRwh2qqnSgWKSaE7Vob35JY4wprvx8ujPZ',
        ],
        time: 1533776547,
        to: {
          address: 'yMi854XzeEmAz9UczDCj9tvXeddweKc9JM',
          amount: '8.99990000',
        },
        txid: '6c42619dd84a02577458ba4f880fe8cfaced9ed518ee7c360c5b107d6ff5b62d',
        type: 'sent',
      },
      {
        from: [
          'yPT2e1oAxN6GEa3tqahKg7KrXkwtKgpgPm',
        ],
        time: 1533766930,
        to: {
          address: 'yeuLv2E9FGF4D9o8vphsaC2Vxoa8ZA7Efp',
          amount: '9.19990000',
        },
        txid: '1240c9e3bba3f143ec354bd37e4b860609b944dee2e426e9868e5c3244e47f04',
        type: 'sent',
      },
      {
        from: [
          'yeuLv2E9FGF4D9o8vphsaC2Vxoa8ZA7Efp',
        ],
        time: 1533535885,
        to: {
          address: 'yPT2e1oAxN6GEa3tqahKg7KrXkwtKgpgPm',
          amount: '10.00000000',
        },
        txid: '1954c3263831dd4d80a9dd8f83a6ce998dae0bed3c9ae111f7c84b0a4f65235f',
        type: 'sent',
      },
      {
        from: [
          'yQxDtKBqQvo3ecMqQVJv7rrZ6PMAGVDNBd',
        ],
        time: 1533815679,
        to: {
          address: 'yiFNYQxfkDxeCBLyWtWQ9w8UGwYugERLCq',
          amount: '50.00000000',
        },
        txid: '4ae8d1960c9a4ed83dbeaf1ad94b4a82f11c8574207144beda87113d94a31da1',
        type: 'receive',
      },
      {
        from: [
          'yeuLv2E9FGF4D9o8vphsaC2Vxoa8ZA7Efp',
        ],
        time: 1533775038,
        to: {
          address: 'yTLjgT7B9PAZXDEvZHWwE4Pyj2MLX1WX2B',
          amount: '8.19980000',
        },
        txid: '1d90ba700b8fa18c8d9a6d3eaa505dde99a4a459c0d1e73bf40ba4b2cc2461cc',
        type: 'sent',
      },
      {
        from: [
          'yTLjgT7B9PAZXDEvZHWwE4Pyj2MLX1WX2B',
        ],
        time: 1533776253,
        to: {
          address: 'yfXQM8TaFiXFYtiFCSm3y6fRq15cj59vVK',
          amount: '6.19970000',
        },
        txid: 'dd02316f28e6d04f1f6f998c30f367dee4dc820309a6cd3cdfc436dc63254c50',
        type: 'sent',
      },
      {
        from: [
          'yfXQM8TaFiXFYtiFCSm3y6fRq15cj59vVK',
        ],
        time: 1533778221,
        to: {
          address: 'yXhm56EBd23RrZpq8WMp1UUUiZobStcaWG',
          amount: '5.19960000',
        },
        txid: 'b452f2d7762b5cd94a0d375e60547c93035b97978a37bcaeed186d27e31feb3a',
        type: 'sent',
      },
      {
        from: [
          'yMi854XzeEmAz9UczDCj9tvXeddweKc9JM',
        ],
        time: 1533776913,
        to: {
          address: 'yN3RXNVbRxA2S4gyHweT9TbFKenuGKd7fW',
          amount: '38.99970000',
        },
        txid: 'd928aedc4ecc6c251cabee0672c19308573e5b4898c32779f3fd211dd8a1fbd8',
        type: 'sent',
      },
      {
        from: [
          'yXhm56EBd23RrZpq8WMp1UUUiZobStcaWG',
        ],
        time: 1533781707,
        to: {
          address: 'ySypFbLpFTXrBbpFqRezwpdwwuaDCfrgpo',
          amount: '4.19950000',
        },
        txid: 'b42c5052d7d31a422e711d50d3754217b0b16b6dfa29cf497b3dd75afa4febcb',
        type: 'sent',
      },
    ];

    return account.getTransactionHistory().then(result => expect(result).to.deep.equal(expected));
  });
  it('should get transaction', () => {
    const expected = {
      txid: 'b42c5052d7d31a422e711d50d3754217b0b16b6dfa29cf497b3dd75afa4febcb',
      version: 1,
      locktime: 0,
      vin: [{
        txid: 'b452f2d7762b5cd94a0d375e60547c93035b97978a37bcaeed186d27e31feb3a', vout: 1, sequence: 4294967295, n: 0, scriptSig: { hex: '483045022100d803161dfdc72be62cb97cc62e862f5c30c061ea987514e7f8e4ff85b5420ed902205c188d20aff9475a86623addf055bed5b8c36e6de2285af1438fb8ede64ae668012103d9576eb9807c16c7bf8e4268969311dd0c1d3209db09e4a62a631b4d68f16fe0', asm: '3045022100d803161dfdc72be62cb97cc62e862f5c30c061ea987514e7f8e4ff85b5420ed902205c188d20aff9475a86623addf055bed5b8c36e6de2285af1438fb8ede64ae668[ALL] 03d9576eb9807c16c7bf8e4268969311dd0c1d3209db09e4a62a631b4d68f16fe0' }, addr: 'yXhm56EBd23RrZpq8WMp1UUUiZobStcaWG', valueSat: 519960000, value: 5.1996, doubleSpentTxID: null,
      }],
      vout: [{
        value: '1.00000000',
        n: 0,
        scriptPubKey: {
          hex: '76a914843859336f31e96025afc658bf152fb0b0bb751188ac', asm: 'OP_DUP OP_HASH160 843859336f31e96025afc658bf152fb0b0bb7511 OP_EQUALVERIFY OP_CHECKSIG', addresses: ['yYNZYgZrCVHQkJ4sPbmegb768zLaoAtREb'], type: 'pubkeyhash',
        },
        spentTxId: '6ca8795f2534972e1371249c3d7b6c5095e1513bc8cc351eeaa2f364020dbc01',
        spentIndex: 4,
        spentHeight: 203674,
      }, {
        value: '4.19950000',
        n: 1,
        scriptPubKey: {
          hex: '76a91449126d84886a9bfc4a2a49aa5ba9cb45c994875288ac', asm: 'OP_DUP OP_HASH160 49126d84886a9bfc4a2a49aa5ba9cb45c9948752 OP_EQUALVERIFY OP_CHECKSIG', addresses: ['ySypFbLpFTXrBbpFqRezwpdwwuaDCfrgpo'], type: 'pubkeyhash',
        },
        spentTxId: null,
        spentIndex: null,
        spentHeight: null,
      }],
      blockhash: '000000000adca9739d088cf532a981d88c301987e40948ffb90bf1c9eeb98ccd',
      blockheight: 203313,
      time: 1533781707,
      blocktime: 1533781707,
      valueOut: 5.1995,
      size: 226,
      valueIn: 5.1996,
      fees: 10000,
      txlock: true,
    };
    return account.getTransaction('b42c5052d7d31a422e711d50d3754217b0b16b6dfa29cf497b3dd75afa4febcb').then(
      data => expect(data).to.be.deep.equal(expected),
    );
  });
  it('should deal with invalid transaction', () => {
    const expected = {};
    return account.getTransaction(1).then(
      data => expect(data).to.be.a('String'),
      err => expect(err).to.be.a('Error').with.property('message', 'Received an invalid txid to fetch : 1'),
    );
    return account.getTransaction(1).then(result => expect(result).to.deep.equal(expected));
  });
  it('should be able to broadcast transaction', () => {
    const { address } = account.getUnusedAddress();
    const txOptsInstant = {
      amount: 10,
      to: address,
      isInstantSend: true,
    };
    const tx = account.createTransaction(txOptsInstant);
    expect(tx).to.equal('0300000001d8fba1d81d21fdf37927c398485b3e570893c17206eeab1c256ccc4edcae28d9000000006a473044022014a6d5b9e6c09edb4040baa8de6b8b21fe7d4f700d2395e4b165040a41c81b930220486ee8ec9c07aaebb50707ce72773927d3095c09369a686c999db587abdb5e5d01210389143a6bee1de5a9583c697e160655c703aa6f7199c93bf14287879a20cd01fbffffffff0200ca9a3b000000001976a914cd5d758c0898175abfbcd12b13ebe783e1dc2b2b88acc0e0d9ac000000001976a914d9e7ddb55be9df9d5678fda4845e9e1181750b9088ac00000000');
    const fakedTx = `${tx}00201010`;

    return account.broadcastTransaction(fakedTx).then(
      data => expect(data).to.be.a('String'),
      err => expect(err).to.be.a('Error').with.property('message', 'Error: Request failed with status code 400'),
    );
  });
  after((done) => {
    account.disconnect();
    account = null;
    wallet = null;
    done();
  });
});
