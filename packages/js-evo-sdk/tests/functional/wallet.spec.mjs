import { wallet } from '../../dist/evo-sdk.module.js';

describe('wallet helpers', () => {
  it('generateMnemonic() returns phrase and validateMnemonic() succeeds', async () => {
    const mnemonic = await wallet.generateMnemonic(12, 'en');
    expect(mnemonic).to.be.a('string');
    expect(await wallet.validateMnemonic(mnemonic, 'en')).to.equal(true);
  });

  it('mnemonicToSeed() returns Uint8Array and derive functions respond', async () => {
    const mnemonic = await wallet.generateMnemonic();
    const seed = await wallet.mnemonicToSeed(mnemonic);
    expect(seed).to.be.instanceOf(Uint8Array);
    expect(await wallet.deriveKeyFromSeedPhrase(mnemonic, null, 'testnet')).to.exist();
    expect(await wallet.deriveKeyFromSeedWithPath(mnemonic, null, "m/44'/5'/0'", 'testnet')).to.exist();
    expect(await wallet.deriveKeyFromSeedWithExtendedPath(mnemonic, null, "m/15'/0'", 'testnet')).to.exist();
  });

  it('key utilities return expected shapes', async () => {
    const kp = await wallet.generateKeyPair('testnet');
    expect(kp).to.be.an('object');
    const kps = await wallet.generateKeyPairs('testnet', 2);
    expect(kps).to.be.an('array');
  });
});
