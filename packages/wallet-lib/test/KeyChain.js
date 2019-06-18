const Dashcore = require('@dashevo/dashcore-lib');
const { expect } = require('chai');
const KeyChain = require('../src/KeyChain');
const { mnemonicToHDPrivateKey } = require('../src/utils/mnemonic');

let keychain;
const mnemonic = 'during develop before curtain hazard rare job language become verb message travel';
const pk = '4226d5e2fe8cbfe6f5beb7adf5a5b08b310f6c4a67fc27826779073be6f5699e';
describe('Keychain', () => {
  it('should create a keychain', () => {
    const expectedException1 = 'Expect privateKey, HDPublicKey or HDPrivateKey';
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
    const hardenedPk = keychain.getHardenedFeaturePath();
    const pk2 = keychain.getKeyForPath('m/44\'/1\'');
    expect(pk2.toString()).to.equal(hardenedPk.toString());
  });
  it('should derive from hardened feature path', () => {
    const hardenedPk = keychain.getHardenedFeaturePath();
    const derivedPk = hardenedPk.deriveChild(0, true).deriveChild(0).deriveChild(0);
    const address = new Dashcore.Address(derivedPk.publicKey.toAddress()).toString();
    expect(address).to.equal('yNfUebksUc5HoSfg8gv98ruC3jUNJUM8pT');
  });
});
