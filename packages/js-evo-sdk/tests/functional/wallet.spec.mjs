import { wallet } from '../../dist/evo-sdk.module.js';

describe('wallet helpers', () => {
  it('generateMnemonic() returns phrase and validateMnemonic() succeeds', () => {
    const mnemonic = wallet.generateMnemonic(12, 'en');
    expect(mnemonic).to.be.a('string');
    expect(wallet.validateMnemonic(mnemonic, 'en')).to.equal(true);
  });

  it('mnemonicToSeed() returns Uint8Array and derive functions respond', () => {
    const mnemonic = wallet.generateMnemonic();
    const seed = wallet.mnemonicToSeed(mnemonic);
    expect(seed).to.be.instanceOf(Uint8Array);
    expect(wallet.deriveKeyFromSeedPhrase(mnemonic, null, 'testnet')).to.exist();
    expect(wallet.deriveKeyFromSeedWithPath(mnemonic, null, "m/44'/5'/0'", 'testnet')).to.exist();
    expect(wallet.deriveKeyFromSeedWithExtendedPath(mnemonic, null, "m/15'/0'", 'testnet')).to.exist();
  });

  it('key utilities return expected shapes', function keyUtilities() {
    const kp = wallet.generateKeyPair('testnet');
    expect(kp).to.be.an('object');
    const kps = wallet.generateKeyPairs('testnet', 2);
    expect(kps).to.be.an('array');
    if (kp?.privateKeyWIF) {
      expect(wallet.keyPairFromWif(kp.privateKeyWIF)).to.be.an('object');
    }
  });

  it('address helpers convert and validate', function addressHelpers() {
    const mnemonic = wallet.generateMnemonic();
    const derived = wallet.deriveKeyFromSeedWithPath(mnemonic, null, "m/44'/5'/0'", 'testnet');
    const { xprv } = derived;
    if (!xprv) {
      this.skip();
    }
    const xpub = wallet.xprvToXpub(xprv);
    const childPub = wallet.deriveChildPublicKey(xpub, 0, false);
    expect(childPub).to.be.a('string');
    const address = wallet.pubkeyToAddress(childPub, 'testnet');
    expect(address).to.be.a('string');
    expect(wallet.validateAddress(address, 'testnet')).to.equal(true);
    const kp = wallet.generateKeyPair('testnet');
    if (kp?.privateKeyWIF) {
      expect(wallet.signMessage('hello', kp.privateKeyWIF)).to.be.a('string');
    }
  });
});
