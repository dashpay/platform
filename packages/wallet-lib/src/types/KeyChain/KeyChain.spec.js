const Dashcore = require('@dashevo/dashcore-lib');
const { expect } = require('chai');
const KeyChain = require('./KeyChain');
const { mnemonicToHDPrivateKey } = require('../../utils/mnemonic');

let keychain;
let keychain2;
const mnemonic = 'during develop before curtain hazard rare job language become verb message travel';
const mnemonic2 = 'birth kingdom trash renew flavor utility donkey gasp regular alert pave layer';
const pk = '4226d5e2fe8cbfe6f5beb7adf5a5b08b310f6c4a67fc27826779073be6f5699e';

const expectedRootDIP15AccountKey_0 = 'tprv8hRzmheQujhJN5XP2dj955nAFCKeEoSifJRWuutdbwWRtusdDQ426jbp75EqErUSuTxmPyxYmP1TpcF5qdxGhXLNXRLMGsRLG6NFCv1WnaQ';
const expectedRootDIP15AccountKey_1 = 'tprv8hRzmheQujhJQyCtFTuUFHxB3Ag5VLB994zhH4CfxbA41cq73HT2mpYq5M33V54oJyn6g514saxxVJB886G55eYX56J6D6x87UNNT6iQHkR';
const expectedKeyForChild_0 = 'tprv8d4podc2Tg459CH2bwLHXj3vdJFBT2rdsk5Nr1djH7hzHdt5LRdvN6QyFwMiDy7ffRdik7fEVRKKgsHB4F18sh8xF6jFXpKq4sUgGBoSbKw';
describe('Keychain', function suite() {
  this.timeout(10000);
  it('should create a keychain', () => {
    const expectedException1 = 'Expect privateKey, publicKey, HDPublicKey, HDPrivateKey or Address';
    expect(() => new KeyChain()).to.throw(expectedException1);
    expect(() => new KeyChain(mnemonic)).to.throw(expectedException1);

    keychain = new KeyChain({ HDPrivateKey: mnemonicToHDPrivateKey(mnemonic, 'testnet') });
    expect(keychain.type).to.equal('HDPrivateKey');
    expect(keychain.network.toString()).to.equal('testnet');
    expect(keychain.keys).to.deep.equal({});

    keychain2 = new KeyChain({ HDPrivateKey: mnemonicToHDPrivateKey(mnemonic2, 'mainnet') });
  });
  it('should get private key', () => {
    expect(keychain.getPrivateKey().toString()).to.equal(pk);
  });
  it('should generate key for full path', () => {
    const path = 'm/44\'/1\'/0\'/0/0';
    const pk2 = keychain.getKeyForPath(path);
    const address = new Dashcore.Address(pk2.publicKey.toAddress()).toString();
    expect(address).to.equal('yNfUebksUc5HoSfg8gv98ruC3jUNJUM8pT');
  });
  it('should get hardened feature path', () => {
    const hardenedPk = keychain.getHardenedBIP44HDKey();
    const pk2 = keychain.getKeyForPath('m/44\'/1\'');
    expect(pk2.toString()).to.equal(hardenedPk.toString());
  });
  it('should get DIP15 account key', function () {
    const rootDIP15AccountKey_0 = keychain.getHardenedDIP15AccountKey(0);
    expect(rootDIP15AccountKey_0.toString()).to.deep.equal(expectedRootDIP15AccountKey_0);
    const rootDIP15AccountKey_1 = keychain.getHardenedDIP15AccountKey(1);
    expect(rootDIP15AccountKey_1.toString()).to.deep.equal(expectedRootDIP15AccountKey_1);
  });
  it('should get DIP15 extended key', function () {
    const userUniqueId = '0x555d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3a';
    const contactUniqueId = '0xa137439f36d04a15474ff7423e4b904a14373fafb37a41db74c84f1dbb5c89b5';

    //  m/9'/5'/15'/0'/0x555d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3a'/0xa137439f36d04a15474ff7423e4b904a14373fafb37a41db74c84f1dbb5c89b5'/0
    const DIP15ExtPubKey_0 = keychain2.getDIP15ExtendedKey(userUniqueId, contactUniqueId, 0, 0, type='HDPublicKey');
    expect(DIP15ExtPubKey_0.toString()).to.equal('xpub6LTkTQFSb8KMgMSz4B6sMZLpkQAY6wSTDprDkHDmLwWLpnjxazuxZn13FrSLKUafitsxuaaffM5a49P6aswhpppWUuYW6eFnwBXshR2W2eY');
    expect(DIP15ExtPubKey_0.publicKey.toString()).to.equal('038030c88ab0106e1f4af3b939db2bafc56f892554106f08da1ce1f9ef10f807bd')

    const DIP15ExtPrivKey_0 = keychain2.getDIP15ExtendedKey(userUniqueId, contactUniqueId, 0, 0);
    expect(DIP15ExtPrivKey_0.toString()).to.equal('xprvA7UQ3tiYkkm4TsNWx9ZrzRQ6CNL3hUibrbvcwtp9nbyMwzQp3Tbi1ygZQaPoigDhCf8XUjMmGK2NbnB2kLXPYg99Lp6e3iki318sdWcFN3q');
    expect(DIP15ExtPrivKey_0.privateKey.toString()).to.equal('fac40790776d171ee1db90899b5eb2df2f7d2aaf35ad56f07ffb8ed2c57f8e60')
    expect(DIP15ExtPrivKey_0.publicKey.toString()).to.equal('038030c88ab0106e1f4af3b939db2bafc56f892554106f08da1ce1f9ef10f807bd')

    const userAhash = "0xa11ce14f698b32e9bb306dba7bbbee831263dcf658abeebb39930460ead117e5";
    const userBhash = "0xb0b052ff075c5ca3c16c3e20e9ac8223834475cc1324ab07889cb24ce6a62793";
    const DIP15ExtKey_1 = keychain.getDIP15ExtendedKey(userAhash, userBhash, 0, 0);
    expect(DIP15ExtKey_1.privateKey.toString()).to.equal('60581b6dca8244d3fb3cfe619b5a22277e5423b01e5285f356981f247e0f4a60')
    expect(DIP15ExtKey_1.publicKey.toString()).to.equal('03deaac00f721151307fbc7bf80d7b8afab98c1f026d67e5f56b21e2013f551ce6')

  });
  it('should derive from hardened feature path', () => {
    const hardenedHDKey = keychain.getHardenedBIP44HDKey();
    const pk2 = keychain.getKeyForPath(`m/44'/1'`);
    expect(pk2.toString()).to.equal(hardenedHDKey.toString());
    expect(hardenedHDKey.toString()).to.deep.equal('tprv8dtrJNytYHRiZY585hmHGbguS6VjGpK49puSB7oXZjLHcQfrAzQkF4ZCxM2DkEbyY85J4EYcZ8EjT5ZCU8ozB727TDdodbfXet5GkGau2RQ');
    const derivedPk = hardenedHDKey.deriveChild(0, true).deriveChild(0).deriveChild(0);
    const address = new Dashcore.Address(derivedPk.publicKey.toAddress()).toString();
    expect(address).to.equal('yNfUebksUc5HoSfg8gv98ruC3jUNJUM8pT');
  });
  it('should get hardened DIP9FeatureHDKey', function () {
    const hardenedHDKey = keychain.getHardenedDIP9FeatureHDKey();
    const pk2 = keychain.getKeyForPath(`m/9'/1'`);
    expect(pk2.toString()).to.equal(hardenedHDKey.toString());
    expect(hardenedHDKey.toString()).to.deep.equal('tprv8fBJjWoGgCpGRCbyzE9RUA59rmoN1RUijhLnXGL4VHnLxvSe523yVg4GrGzbR6TyXtdynAEh5z8UX55EXt2Cb3xjvrsx2PgTY9BHxzFVkWn');
  });
  it('should generate key for child', () => {
    const keychain2 = new KeyChain({ HDPrivateKey: mnemonicToHDPrivateKey(mnemonic, 'testnet') });
    const keyForChild = keychain2.generateKeyForChild(0);
    expect(keyForChild.toString()).to.equal(expectedKeyForChild_0);
  });


  it('should sign', () => {

  });
});
describe('Keychain - clone', function suite() {
  this.timeout(10000);
  it('should clone', () => {
    const keychain2 = new KeyChain(keychain);
    expect(keychain2).to.deep.equal(keychain);
    expect(keychain2.keys).to.deep.equal(keychain.keys);
  });
});
describe('Keychain - single privateKey', function suite() {
  this.timeout(10000);
  it('should correctly errors out when not a HDPublicKey (privateKey)', () => {
    const privateKey = Dashcore.PrivateKey().toString();
    const network = 'livenet';
    const pkKeyChain = new KeyChain({ privateKey, network });
    expect(pkKeyChain.network).to.equal(network);
    expect(pkKeyChain.keys).to.deep.equal({});
    expect(pkKeyChain.type).to.equal('privateKey');
    expect(pkKeyChain.privateKey).to.equal(privateKey);

    const expectedException1 = 'Wallet is not loaded from a mnemonic or a HDPubKey, impossible to derivate keys';
    const expectedException2 = 'Wallet is not loaded from a mnemonic or a HDPubKey, impossible to derivate child';
    expect(() => pkKeyChain.generateKeyForPath()).to.throw(expectedException1);
    expect(() => pkKeyChain.generateKeyForChild()).to.throw(expectedException2);
  });
  it('should get private key', () => {
    const privateKey = Dashcore.PrivateKey().toString();
    const pkKeyChain = new KeyChain({ privateKey, network: 'livenet' });
    expect(pkKeyChain.getPrivateKey().toString()).to.equal(privateKey);
  });
});
