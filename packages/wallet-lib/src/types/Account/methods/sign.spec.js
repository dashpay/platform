const { expect } = require('chai');
const Dashcore = require('@dashevo/dashcore-lib');
const { Wallet } = require('../../../index');


describe('Account - sign', function suite() {
  this.timeout(10000);
  let wallet;
  let account;
  beforeEach(async () => {
    wallet = new Wallet({ offlineMode: true });
    account = await wallet.getAccount({ index: 0 });
  });

  afterEach(() => {
    wallet.disconnect();
  });
  it('should sign and verify a message', () => {
    const idKey = account.getIdentityHDKey();
    const idPrivateKey = idKey.privateKey;
    const idAddress = idPrivateKey.toAddress().toString();
    const message = new Dashcore.Message('hello, world');
    const signed = account.sign(message, idPrivateKey);
    const verify = message.verify(idAddress, signed.toString());
    expect(verify).to.equal(true);
  });
});
