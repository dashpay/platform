const { expect } = require('chai');
const Dashcore = require('@dashevo/dashcore-lib');
const { Wallet } = require('../../../index');

const transactions = {"4e2a8b05a805fcee959b8ecfd5557e196a9b8490dd280d6f599b391d650407c8":{"hash":"4e2a8b05a805fcee959b8ecfd5557e196a9b8490dd280d6f599b391d650407c8","version":2,"inputs":[{"prevTxId":"1a38feca081f5c03a4fedbb62eda8706b2b43d5fc13c60b0c84e0d2c67877a4c","outputIndex":1,"sequenceNumber":4294967294,"script":"4730440220297261672847b242b46f860a827d4ef6d392471739937fd19c9e5337dd8abcfc02206f2d3fc3485e8163ac470f67dcad52c650356dc797cae8832e50afd881eba257012103a65caff6ca4c0415a3ac182dfc2a6d3a4dceb98e8b831e71501df38aa156f2c1","scriptString":"71 0x30440220297261672847b242b46f860a827d4ef6d392471739937fd19c9e5337dd8abcfc02206f2d3fc3485e8163ac470f67dcad52c650356dc797cae8832e50afd881eba25701 33 0x03a65caff6ca4c0415a3ac182dfc2a6d3a4dceb98e8b831e71501df38aa156f2c1"}],"outputs":[{"satoshis":187719774,"script":"76a91489e98f97403bb0b5674f27177452c4e8980ac42488ac"},{"satoshis":12312280000,"script":"76a91427da1075bef6c1e932f5caab1ded021f2acb65e188ac"}],"nLockTime":13621}};
const mnemonic = 'wrist ladder salute build walk other scrap stumble true hotel layer treat';

describe('Account - sign', function suite() {
  this.timeout(10000);
  let wallet;
  let account;
  beforeEach(async () => {
    wallet = new Wallet({ mnemonic, offlineMode: true });
    account = await wallet.getAccount({ index: 0 });
  });

  afterEach(() => {
    wallet.disconnect();
  });
  it('should sign a transaction with a message', function () {
    account.importTransactions(transactions);
    const transaction = account.createTransaction({
      recipient: 'yNPbcFfabtNmmxKdGwhHomdYfVs6gikbPf', // Evonet faucet
      satoshis: 1000000, // 1 Dash
    });
    const data = 'correct horse battery staple';
    transaction.addData(data);
    const signedTransaction = account.sign(transaction);
    expect(signedTransaction.isFullySigned()).to.equal(true);
    expect(Buffer.from(transaction.toJSON().outputs[1].script, 'hex').slice(2).toString('utf-8')).to.equal(data);
  });
  it('should sign and verify a message', () => {
    const idKey = account.identities.getIdentityHDKeyByIndex(0, 0);
    const idPrivateKey = idKey.privateKey;
    const idAddress = idPrivateKey.toAddress().toString();
    const message = new Dashcore.Message('hello, world');
    const signed = account.sign(message, idPrivateKey);
    const verify = message.verify(idAddress, signed.toString());
    expect(verify).to.equal(true);
  });
});
