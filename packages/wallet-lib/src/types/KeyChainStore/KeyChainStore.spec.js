const {HDPrivateKey} = require("@dashevo/dashcore-lib");
const KeyChainsStore = require('./KeyChainStore');
const KeyChain = require("../KeyChain/KeyChain");
const { expect } = require('chai');

describe('KeyChainStore', function suite() {
  let keyChainsStore;
  let hdPrivateKey = new HDPrivateKey()
  let hdPublicKey = new HDPrivateKey().hdPublicKey
  let keyChain = new KeyChain({HDPrivateKey: hdPrivateKey})
  let keyChainPublic = new KeyChain({HDPublicKey: hdPublicKey})
  let walletKeyChain = new KeyChain({HDPrivateKey:new HDPrivateKey()});
  it('should create a KeyChainStore', () => {
    keyChainsStore = new KeyChainsStore();
    expect(keyChainsStore).to.exist;
    expect(keyChainsStore.keyChains).to.be.a('Map')
  });
  it('should be able to add a keyChain', function () {
    keyChainsStore.addKeyChain(keyChain)
    expect(keyChainsStore.keyChains.has(keyChain.keyChainId)).to.equal(true);
    keyChainsStore.addKeyChain(keyChainPublic)
    expect(keyChainsStore.keyChains.has(keyChainPublic.keyChainId)).to.equal(true);
  });
  it('should allow to specify a specific master keychain', function () {
    keyChainsStore.addKeyChain(walletKeyChain, { isMasterKeyChain: true });
    expect(keyChainsStore.keyChains.has(walletKeyChain.keyChainId)).to.equal(true);
  });
  it('should get all keyChains', function () {
    const keyChains = keyChainsStore.getKeyChains()
    expect(keyChains).to.deep.equal([keyChain, keyChainPublic, walletKeyChain]);
  });
  it('should get a keychain by its ID', () => {
    const requestedKeychain = keyChainsStore.getKeyChain(keyChainPublic.keyChainId);
    expect(requestedKeychain).to.equal(keyChainPublic);
  })
  it('should get a master keychain', function () {
    const requestedWalletKeyChain = keyChainsStore.getMasterKeyChain();
    expect(requestedWalletKeyChain).to.equal(walletKeyChain);
  });
  it('should make a child key chain store', function () {
    const childKeyChainStore = keyChainsStore.makeChildKeyChainStore('m/0')
    expect(childKeyChainStore).to.exist;
    expect(childKeyChainStore.keyChains).to.be.a('Map')
    expect(childKeyChainStore.getMasterKeyChain().rootKeyType).to.be.equal(HDPrivateKey.name)
  });
});

