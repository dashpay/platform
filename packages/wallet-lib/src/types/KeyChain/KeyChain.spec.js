const Dashcore = require('@dashevo/dashcore-lib');
const { expect } = require('chai');
const KeyChain = require('./KeyChain');
const { mnemonicToHDPrivateKey } = require('../../utils/mnemonic');

let keychain;
const mnemonic = 'during develop before curtain hazard rare job language become verb message travel';
const pk = '4226d5e2fe8cbfe6f5beb7adf5a5b08b310f6c4a67fc27826779073be6f5699e';
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
    const hardenedPk = keychain.getHardenedBIP44Path();
    const pk2 = keychain.getKeyForPath('m/44\'/1\'');
    expect(pk2.toString()).to.equal(hardenedPk.toString());
  });
  it('should derive from hardened feature path', () => {
    const hardenedPk = keychain.getHardenedBIP44Path();
    const derivedPk = hardenedPk.deriveChild(0, true).deriveChild(0).deriveChild(0);
    const address = new Dashcore.Address(derivedPk.publicKey.toAddress()).toString();
    expect(address).to.equal('yNfUebksUc5HoSfg8gv98ruC3jUNJUM8pT');
  });
  it('should generate key for child', () => {
    const keychain2 = new KeyChain({ HDPrivateKey: mnemonicToHDPrivateKey(mnemonic, 'testnet') });
    const keyForChild = keychain2.generateKeyForChild(0);
    expect(keyForChild.toString()).to.equal('tprv8d4podc2Tg459CH2bwLHXj3vdJFBT2rdsk5Nr1djH7hzHdt5LRdvN6QyFwMiDy7ffRdik7fEVRKKgsHB4F18sh8xF6jFXpKq4sUgGBoSbKw');
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
