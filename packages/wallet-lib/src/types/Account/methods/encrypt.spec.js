const { expect } = require('chai');
const CryptoJS = require('crypto-js');
const { Wallet } = require('../../../index');

const derivationPath = "m/44'/1'/0'/0";

describe('Account - encrypt',  function suite() {
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
  const jsonObject = {
    string: 'string',
    list: ['a', 'b', 'c', 'd'],
    obj: {
      int: 1,
      boolean: true,
      theNull: null,
    },
  };

  const secret = 'secret';

  it('should encrypt extPubKey with aes', () => {
    const extPubKey = account.keyChain.getKeyForPath(derivationPath, 'HDPublicKey').toString();
    const encryptedExtPubKey = account.encrypt('aes', extPubKey, secret).toString();
    const bytes = CryptoJS.AES.decrypt(encryptedExtPubKey, secret);
    const decrypted = bytes.toString(CryptoJS.enc.Utf8);
    expect(decrypted).to.equal(extPubKey);
  });
  it('should encrypt a encoded json', () => {
    const encodedJSON = account.encode('cbor', jsonObject).toString('hex');
    const encryptedJSON = account.encrypt('aes', encodedJSON, secret);

    const decryptedEncodedJSON = CryptoJS
      .AES
      .decrypt(encryptedJSON, secret)
      .toString(CryptoJS.enc.Utf8);

    const decodedJSON = account.decode('cbor', decryptedEncodedJSON);
    expect(encodedJSON).to.equal(decryptedEncodedJSON);
    expect(decodedJSON).to.deep.equal(jsonObject);
  });
});
