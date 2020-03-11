//FIXME : Right now, it gets executed by CI in parallel on multiple which is annoying.
return;
const { expect } = require('chai');
const { Wallet } = require('../../src/index');
const { expectThrowsAsync } = require('../test.utils');
const sweepWallet = require('../../src/types/Wallet/methods/sweepWallet');

const paperWallet = {
  publicKey: 'ybvbBPisVjiemj4qSg1mzZAzTSAPk64Ppf',
  privateKey: 'XE6ZTNwkjyuryGho75fAfCBBtL8rMy9ttLq1ANLF1TmMo2zwZXHq',
};
const testnetPaperWallet = {
  publicKey: 'yiqbNC2EEpNgAUC3XrJDccEZxzGsf2rc9w',
  privateKey: 'cUWeuzNzpVfKTQ28NEGaZWB3Sck3pDRNd6mSS1zqnMuPXBDefgkS',
};

const toBeSweptPaperWallet = {
  publicKey: 'ydRMs2ZoPHFHfJR1thUNb34zR8TkXta9uQ',
  privateKey: 'aa37843180c2690483d44cd211b06239e21cd82e25f5e809586d151d53d112f9',
};
const toBeSweptPaperWallet2 = {
  publicKey: 'ybye8CrNahooxLqFucWwTexUd85J5HEbv1',
  privateKey: '66fe6ed3bab6be3c6d1fcab99d704287db9dd24b53216b68ec39a0fa6641abb1',
};
describe('Wallet - sweepWallet', function suite() {
  this.timeout(50000);
  let emptyAccount;
  let fullAccount;
  let toBeSweptAccount;
  let toBeSweptAccount2;
  let emptyWallet;
  let fullWallet;
  let toBeSweptWallet;
  let toBeSweptWallet2;
  let newWallet;
  let mnemonic;
  before(async () => {
    emptyWallet = new Wallet({
      privateKey: paperWallet.privateKey,
      network: 'testnet',
      transporter: { type: 'DAPIClient', devnetName: 'palinka' },
    });
    emptyAccount = emptyWallet.getAccount();
    await emptyAccount.isReady();

    fullWallet = new Wallet({
      privateKey: testnetPaperWallet.privateKey,
      network: 'testnet',
      transporter: { type: 'DAPIClient', devnetName: 'palinka' },
    });
    fullAccount = fullWallet.getAccount();
    await fullAccount.isReady();
  });
  it('should pass sanitary check', () => {
    const addr = emptyAccount.getAddress();
    expect(addr).to.deep.equal({
      path: '0',
      index: 0,
      address: paperWallet.publicKey,
      transactions: [],
      balanceSat: 0,
      unconfirmedBalanceSat: 0,
      utxos: {},
      fetchedLast: 0,
      used: false,
    });
    const addrTestnet = fullAccount.getAddress();
    expect(addrTestnet.path).to.equal('0');
    expect(addrTestnet.index).to.equal(0);
    expect(addrTestnet.address).to.equal(testnetPaperWallet.publicKey);
    // If this is not passing, fund this : yiqbNC2EEpNgAUC3XrJDccEZxzGsf2rc9w :)
    expect(addrTestnet.used).to.equal(true);
  });
  it('should warn on empty balance', () => {
    const exceptedException = 'Cannot sweep an empty private key (current balance: 0)';
    expectThrowsAsync(async () => await emptyWallet.sweepWallet(), exceptedException);
  });
  it('should warn on sweep from mnemonic', async () => {
    const exceptedException = 'Can only sweep wallet initialized from privateKey';


    const mockWallet = {
      walletType: 'HDWALLET',
      getAccount: () => ({ getAddress: () => ({ address: null }), isReady: () => true }),
    };
    expectThrowsAsync(async () => await sweepWallet.call(mockWallet), exceptedException);
  });
  it('should prepare', (done) => {
    const satoshis = 2500;
    if (fullAccount.getTotalBalance() <= satoshis) throw new Error('Fund me!');
    const addr = toBeSweptPaperWallet.publicKey;
    const addr2 = toBeSweptPaperWallet2.publicKey;
    console.log(`Preparing Sweep wallet - Sending ${satoshis} to Wallet1 ${addr}`);
    console.log(`Preparing Sweep wallet - Sending ${satoshis} to Wallet2 ${addr2}`);
    (async () => {
      const txid = await fullAccount.broadcastTransaction(fullAccount.createTransaction({ recipient: addr, satoshis }));
      console.log(`TxID to Wallet1: ${txid}`);
      const txid2 = await fullAccount.broadcastTransaction(fullAccount.createTransaction({ recipient: addr2, satoshis }));
      console.log(`TxID to Wallet2: ${txid2}`);
      setTimeout(async () => {
        toBeSweptWallet = new Wallet({
          privateKey: toBeSweptPaperWallet.privateKey,
          network: 'testnet',
          transporter: { type: 'DAPIClient', devnetName: 'palinka' },
        });
        toBeSweptAccount = toBeSweptWallet.getAccount();
        await toBeSweptAccount.isReady();
        const balance = toBeSweptAccount.getTotalBalance();
        console.log(`Sweepable Wallet contains - ${balance}`);
        toBeSweptWallet2 = new Wallet({
          privateKey: toBeSweptPaperWallet2.privateKey,
          network: 'testnet',
          transporter: { type: 'DAPIClient', devnetName: 'palinka' },
        });
        toBeSweptAccount2 = toBeSweptWallet2.getAccount();
        await toBeSweptAccount2.isReady();
        const balance2 = toBeSweptAccount2.getTotalBalance();
        console.log(`Sweepable Wallet2 contains - ${balance2}`);

        if (balance < 2500) throw new Error('Failed to fund wallet1');
        if (balance2 < 2500) throw new Error('Failed to fund wallet2');
        done();
      }, 5000);
    })();
  });
  it('should work', async () => {
    newWallet = await toBeSweptWallet.sweepWallet();
    mnemonic = newWallet.exportWallet();
    console.log(`Swept wallet1 to new wallet ${mnemonic}`);
    await toBeSweptWallet2.sweepWallet({ mnemonic });
    console.log(`Swept wallet2 to same wallet ${mnemonic}`);
    const newAcc = newWallet.getAccount();
    await newAcc.isReady();
    const newAddr = newAcc.getAddress();
    const balance = newAcc.getTotalBalance();
    console.log(`New address : ${newAddr.address} = Balance: ${balance}`);
  });
  it('should verify', async () => {
    const walletVerification = new Wallet({
      mnemonic,
      network: 'testnet',
      transporter: { type: 'DAPIClient', devnetName: 'palinka' },
    });
    const account = walletVerification.getAccount({ index: 0 });
    await account.isReady();
    const balance = account.getTotalBalance();
    expect(balance).to.be.gte(4500);
    console.log(balance);
  });
  after(() => {
    emptyWallet.disconnect();
    fullWallet.disconnect();
    toBeSweptWallet.disconnect();
    toBeSweptWallet2.disconnect();
    newWallet.disconnect();
  });
});
