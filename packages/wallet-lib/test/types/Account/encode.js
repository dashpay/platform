const { expect } = require('chai');
const cbor = require('cbor');
const { Wallet } = require('../../../src');

const derivationPath = "m/44'/1'/0'/0";

describe('Account - encrypt', () => {
  let wallet;
  let account;
  beforeEach(() => {
    wallet = new Wallet();
    account = wallet.getAccount(0);
  });

  afterEach(() => {
    wallet.disconnect();
  });

  it('should encode extPubKey with cbor', () => {
    const extPubKey = account.keyChain.getKeyForPath(derivationPath, 'HDPublicKey').toString();
    const encryptedExtPubKey = account.encode('cbor', extPubKey);
    const decrypted = cbor.decodeFirstSync(encryptedExtPubKey);
    expect(decrypted).to.equal(extPubKey);
  });
});
