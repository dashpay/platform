const { expect } = require('chai');
const { Wallet, EVENTS } = require('../../src');

let wallet;
let account;

describe('Integration - fromMnemonic', function suite() {
  this.timeout(50000);

  it('should init a wallet', () => {
    wallet = new Wallet({
      mnemonic: 'local mom grief muffin hurdle stamp glue train satoshi kitchen damage cliff',
      network: 'testnet',
    });
    expect(wallet.network).to.equal('testnet');
    expect(wallet.walletType).to.equal('hdwallet');
    expect(wallet.walletId).to.equal('3f51373e24');
    expect(wallet.keyChain.network.toString()).to.equal('testnet');
    expect(wallet.keyChain.HDPrivateKey.toString()).to.equal('tprv8ZgxMBicQKsPdjb7zbZWY6p8eJKaGCcMvB6GmgLx9UQJvZVRipMfzMxF3A8SPbWPmjSVgsyu3Euyk9Puj7P2xKVJUJRAqpr1uR65wxRkk3m');
  });
  it('should get an account', (done) => {
    account = wallet.getAccount({ index: 0 });
    account.events.on(EVENTS.READY, done);
  });
  it('should get addresses', () => {
    expect(account.getAddress(0).address).to.equal('yPemRQzMffYKvDBDXW5Mt64wUPi5ZNdpFW');
    // If not, we might have got a reset from backend, just fund that above address :)
    expect(account.getUnusedAddress().address).to.not.equal('yPemRQzMffYKvDBDXW5Mt64wUPi5ZNdpFW');
  });
  it('should get utxos', () => {
    const utxoSet = account.getUTXOS();
    expect(utxoSet.length > 0).to.equal(true);
  });
  it('should get a coinSelection to create a new tx', () => {
    const tx = account.createTransaction({ recipient: 'yPemRQzMffYKvDBDXW5Mt64wUPi5ZNdpFW', satoshis: 10000 });
    expect(tx._inputAmount >= 10000).to.equal(true);
    // If fee are above 1000 there is an issue we want to raise here.
    expect(tx._fee < 1000).to.equal(true);
  });
  it('should disconnect', () => {
    wallet.disconnect();
  });
});
