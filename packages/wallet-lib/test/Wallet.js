const { expect } = require('chai');
const Dashcore = require('@dashevo/dashcore-lib');
const Mnemonic = require('@dashevo/dashcore-mnemonic');
const { Wallet } = require('../src/index');
const { mnemonicString1, invalidMnemonicString1 } = require('./fixtures.json');

const mnemonic1 = new Mnemonic(mnemonicString1);
const privateHDKey1 = mnemonic1.toHDPrivateKey('', 'testnet');


describe('Wallet', () => {
  it('should create a wallet from a HDPrivateKey', () => {
    const network = 'testnet';
    const config = {
      seed: privateHDKey1,
      network,
    };
    const wallet = new Wallet(config);
    // eslint-disable-next-line no-unused-expressions
    expect(wallet).to.exist;
    expect(wallet).to.be.a('object');
    expect(wallet.constructor.name).to.equal('Wallet');
    expect(wallet.transport).to.not.equal(null);
    expect(wallet.HDPrivateKey.toString()).to.equal(privateHDKey1.toString());
    wallet.disconnect();
  });
  it('should create a wallet from a mnemonic string', () => {
    const network = Dashcore.Networks.testnet;
    const config = {
      mnemonic: mnemonicString1,
      network,
    };
    const wallet = new Wallet(config);
    const hdKey = mnemonic1.toHDPrivateKey('', network);
    // eslint-disable-next-line no-unused-expressions
    expect(wallet).to.exist;
    expect(wallet).to.be.a('object');
    expect(wallet.constructor.name).to.equal('Wallet');
    expect(wallet.transport).to.not.equal(null);
    expect(wallet.HDPrivateKey.toString()).to.equal(hdKey.toString());
    wallet.disconnect();
  });
  it('should create a wallet from a mnemonic object', () => {
    const network = Dashcore.Networks.testnet;
    const config = {
      mnemonic: mnemonic1,
      network,
    };
    const wallet = new Wallet(config);
    const hdKey = mnemonic1.toHDPrivateKey('', network);
    // eslint-disable-next-line no-unused-expressions
    expect(wallet).to.exist;
    expect(wallet).to.be.a('object');
    expect(wallet.constructor.name).to.equal('Wallet');
    expect(wallet.transport).to.not.equal(null);
    expect(wallet.HDPrivateKey.toString()).to.equal(hdKey.toString());
    wallet.disconnect();
  });
  it('should create a wallet when mnemonic not set', () => {
    const network = Dashcore.Networks.testnet;
    const config = {
      network,
    };
    const wallet = new Wallet(config);
    // eslint-disable-next-line no-unused-expressions
    expect(wallet).to.exist;
    expect(wallet).to.be.a('object');
    expect(wallet.constructor.name).to.equal('Wallet');
    expect(wallet.transport).to.not.equal(null);
    expect(wallet.mnemonic).to.be.an.instanceof(Mnemonic);
    wallet.disconnect();
  });
  it('should be able to create a wallet', () => {
    const wallet = new Wallet({
      mnemonic: mnemonicString1,
      network: Dashcore.Networks.testnet,
    });

    const acc1 = wallet.createAccount({ mode: 'light' });
    const acc2 = wallet.createAccount({ mode: 'light' });

    [acc1, acc2].forEach((el, i) => {
      // eslint-disable-next-line no-unused-expressions
      expect(el).to.exist;
      expect(el).to.be.a('object');
      expect(el.constructor.name).to.equal('Account');
      expect(el.BIP44PATH).to.equal(`m/44'/1'/${i}'`);
    });
    acc1.disconnect();
    acc2.disconnect();
    wallet.disconnect();
  });

  it('should not be able to getAddressSummary with fake transport', () => {
    const network = 'testnet';
    const config = {
      seed: privateHDKey1,
      network,
      transport: 'fake',
    };
    const wallet = new Wallet(config);
    // eslint-disable-next-line no-unused-expressions
    expect(wallet).to.exist;
    expect(wallet).to.be.a('object');
    expect(wallet.constructor.name).to.equal('Wallet');

    expect(wallet.transport).to.not.equal(null);
    expect(wallet.transport).to.be.a('object');
    expect(wallet.transport.valid).to.equal(false);
    expect(wallet.transport.type).to.equal('String');

    expect(wallet.HDPrivateKey.toString()).to.equal(privateHDKey1.toString());
    wallet.transport.getAddressSummary('fake').then(
      () => Promise.reject(new Error('Expected method to reject.')),
      err => expect(err).to.be.a('Error').with.property('message', 'this.transport.getAddressSummary is not a function'),
    );
    wallet.disconnect();
  });

  it('should not be able to getAddressSummary with invalid address', () => {
    const network = 'testnet';
    const config = {
      seed: privateHDKey1,
      network,
      transport: 'fake',
    };
    const wallet = new Wallet(config);
    // eslint-disable-next-line no-unused-expressions
    expect(wallet).to.exist;
    expect(wallet).to.be.a('object');
    expect(wallet.constructor.name).to.equal('Wallet');
    expect(wallet.transport).to.not.equal(null);
    expect(wallet.transport).to.be.a('object');
    expect(wallet.transport.valid).to.equal(false);
    expect(wallet.transport.type).to.equal('String');
    expect(wallet.HDPrivateKey.toString()).to.equal(privateHDKey1.toString());
    wallet.transport.getAddressSummary(123).then(
      () => Promise.reject(new Error('Expected method to reject.')),
      err => expect(err).to.be.a('Error').with.property('message', 'Received an invalid address to fetch'),
    );
    wallet.disconnect();
  });

  it('should reject without network', () => {
    const conf = {
      mnemonic: mnemonic1,
    };
    expect(() => new Wallet(conf)).to.throw('Expected a valid network (typeof Network or String');
  });
  it('should reject with invalid mnemonic: true', () => {
    const conf = {
      mnemonic: true,
      network: Dashcore.Networks.testnet,
    };
    expect(() => new Wallet(conf)).to.throw('Expected a valid mnemonic (typeof String or Mnemonic)');
  });
  it('should reject with invalid mnemonic: false', () => {
    const conf = {
      mnemonic: false,
      network: Dashcore.Networks.testnet,
    };
    expect(() => new Wallet(conf)).to.throw('Expected a valid mnemonic (typeof String or Mnemonic)');
  });
  it('should reject with invalid mnemonic: 0', () => {
    const conf = {
      mnemonic: false,
      network: Dashcore.Networks.testnet,
    };
    expect(() => new Wallet(conf)).to.throw('Expected a valid mnemonic (typeof String or Mnemonic)');
  });
  it('should reject with invalid seed: true', () => {
    const network = 'testnet';
    const conf = {
      seed: true,
      network,
    };
    expect(() => new Wallet(conf)).to.throw('Expected a valid seed (typeof HDPrivateKey or String)');
  });
  it('should reject with invalid seed: false', () => {
    const network = 'testnet';
    const conf = {
      seed: false,
      network,
    };
    expect(() => new Wallet(conf)).to.throw('Expected a valid seed (typeof HDPrivateKey or String)');
  });
  it('should reject with invalid seed: 0', () => {
    const network = 'testnet';
    const conf = {
      seed: 0,
      network,
    };
    expect(() => new Wallet(conf)).to.throw('Expected a valid seed (typeof HDPrivateKey or String)');
  });


  it('should reject invalid mnemonic', () => {
    const conf = {
      mnemonic: invalidMnemonicString1,
      network: Dashcore.Networks.testnet,
    };
    expect(() => new Wallet(conf)).to.throw('Mnemonic string is invalid: knife easily prosper input concert merge prepare autumn pen blood glance chair');
  });
  it('should be able to export to a HDPrivKey', () => {
    const network = Dashcore.Networks.testnet;
    const config = {
      mnemonic: mnemonic1,
      network,
    };
    const walletTestnet = new Wallet(config);
    const exported = walletTestnet.exportWallet(true);
    expect(exported).to.deep.equal(privateHDKey1);
    walletTestnet.disconnect();
  });
  it('should be able to export a mnemonic', () => {
    const network = Dashcore.Networks.testnet;
    const config = {
      mnemonic: mnemonic1,
      network,
    };

    const walletTestnet = new Wallet(config);
    const exported = walletTestnet.exportWallet();
    expect(exported).to.equal(mnemonicString1);
    walletTestnet.disconnect();
  });
  it('should be able to export to a HDPrivKey with seed', () => {
    const network = 'testnet';
    const config = {
      seed: privateHDKey1,
      network,
    };
    const walletTestnet = new Wallet(config);
    const exported = walletTestnet.exportWallet(true);
    expect(exported).to.deep.equal(privateHDKey1);
    walletTestnet.disconnect();
  });
  it('should not be able to export with seed', () => {
    const network = 'testnet';
    const config = {
      seed: privateHDKey1,
      network,
    };
    const walletTestnet = new Wallet(config);
    expect(() => walletTestnet.exportWallet()).to.throw('Wallet was not initiated with a mnemonic, can\'t export it');
    walletTestnet.disconnect();
  });
  it('should encrypt wallet with a passphrase', () => {
    const network = Dashcore.Networks.testnet;
    const passphrase = 'Evolution';
    const config = {
      mnemonic: mnemonic1,
      passphrase,
      network,
    };
    const walletTestnet = new Wallet(config);
    const encryptedHDPriv = walletTestnet.exportWallet(true);
    const expectedHDPriv = 'tprv8ZgxMBicQKsPd5PxuGP2oSibQ3uXZBVBYePFjZmVSz5urXdyoJSzsZq9SrTDNRE5e5n3FnRMWDbt4foEJejiDCGooDBu7GSajSonqDcdazh';
    expect(encryptedHDPriv.toString()).to.equal(expectedHDPriv);
    walletTestnet.disconnect();
  });
  it('should be able to create an account at a specific index', () => {
    const network = Dashcore.Networks.testnet;
    const passphrase = 'Evolution';
    const config = {
      mnemonic: mnemonic1,
      passphrase,
      network,
    };
    const walletTestnet = new Wallet(config);
    const account = walletTestnet.createAccount();
    // eslint-disable-next-line no-unused-expressions
    expect(account).to.exist;
    const accountSpecificIndex = walletTestnet.createAccount({ accountIndex: 42 });
    expect(accountSpecificIndex.BIP44PATH.split('/')[3]).to.equal('42\'');
    expect(accountSpecificIndex.accountIndex).to.equal(42);
    walletTestnet.disconnect();
  });
  it('should be able to get an account at a specific index', () => {
    const network = Dashcore.Networks.testnet;
    const passphrase = 'Evolution';
    const config = {
      mnemonic: mnemonic1,
      passphrase,
      network,
    };
    const walletTestnet = new Wallet(config);
    const account = walletTestnet.getAccount();
    expect(account.accountIndex).to.equal(0);

    const nonAlreadyCreatedAccount = walletTestnet.getAccount(41);

    expect(nonAlreadyCreatedAccount.accountIndex).to.equal(41);

    const accountSpecific = walletTestnet.getAccount(42);
    expect(accountSpecific.accountIndex).to.equal(42);

    expect(walletTestnet.accounts.length).to.equal(3);
    walletTestnet.disconnect();
  });
  it('should be able to import a cache', () => {
    const network = Dashcore.Networks.testnet;
    const passphrase = 'Evolution';
    const config = {
      mnemonic: mnemonic1,
      passphrase,
      network,
      cache: {
        transactions: {
          '9ab39713e9ce713d41ca6974db83e57bced02402e9516b8a662ed60d5c08f6d1': {
            blockhash: '000000000a84c4703da7a69cfa65837251e4aac80e1621f2a2cc9504e0c149ba',
            blockheight: 201436,
            blocktime: 1533525448,
            fees: 1000,
            size: 225,
            txid: '9ab39713e9ce713d41ca6974db83e57bced02402e9516b8a662ed60d5c08f6d1',
            txlock: true,
            vin: [{
              txid: 'e4524e918977b70ab47160d8e3b87a5fa9f88f22e43f0eec2abbee2cf364c93b', vout: 1, sequence: 4294967295, n: 0, scriptSig: { hex: '4730440220154e37879e70784daff6cf04993cc88e8cf7e5357f82e98df9c117941cd5b3f702200d02f583085fbfd28c77d31c6b9a69f641a8fef5e99aaec8b8aedd4a3326e4100121025deea4fcd79eb876daa0f5829659c76f00f6b3fe6bf12e3ea83ecc763219bf88', asm: '30440220154e37879e70784daff6cf04993cc88e8cf7e5357f82e98df9c117941cd5b3f702200d02f583085fbfd28c77d31c6b9a69f641a8fef5e99aaec8b8aedd4a3326e410[ALL] 025deea4fcd79eb876daa0f5829659c76f00f6b3fe6bf12e3ea83ecc763219bf88' }, addr: 'yhzoBe1aCTTganFBzFb3ErF4ufwMqonK5a', valueSat: 81246619083, value: 812.46619083, doubleSpentTxID: null,
            }],
            vout: [{
              value: '2.00000000',
              spentTxId: '6b90bf01b10a0c6cac018d376823f6b330edf2cbb783cc3d02004f8706bbc311',
              spentIndex: 7,
            }, {
              value: '810.46609083',
              spentTxId: '22c368e09ad8b36553b383c6a4ae989f91d1f66622b2b685262580c8a45175a4',
              spentIndex: 0,
            }],
          },
        },
        addresses: {
          "m/44'/1'/0'/0/19": {
            address: 'yLmv6uX1jmn14pCDpc83YCsA8wHVtcbaNw',
            balance: 0,
            fetchedLast: 1533527600644,
            path: "m/44'/1'/0'/0/19",
            transactions: [],
            utxos: [],
          },
        },
      },
    };
    const walletTestnet = new Wallet(config);
    const account = walletTestnet.getAccount();
    expect(account.accountIndex).to.equal(0);

    const nonAlreadyCreatedAccount = walletTestnet.getAccount(41);

    expect(nonAlreadyCreatedAccount.accountIndex).to.equal(41);

    const accountSpecific = walletTestnet.getAccount(42);
    expect(accountSpecific.accountIndex).to.equal(42);

    expect(walletTestnet.accounts.length).to.equal(3);
    walletTestnet.disconnect();
  });
});
