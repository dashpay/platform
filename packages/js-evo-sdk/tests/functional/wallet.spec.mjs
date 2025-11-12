import { wallet } from '../../dist/evo-sdk.module.js';

describe('wallet helpers', () => {
  it('generateMnemonic() returns phrase and validateMnemonic() succeeds', async () => {
    const mnemonic = await wallet.generateMnemonic({ wordCount: 12, languageCode: 'en' });
    expect(mnemonic).to.be.a('string');
    expect(await wallet.validateMnemonic(mnemonic, 'en')).to.equal(true);
  });

  it('mnemonicToSeed() returns Uint8Array and derive functions respond', async () => {
    const mnemonic = await wallet.generateMnemonic();
    const seed = await wallet.mnemonicToSeed(mnemonic);
    expect(seed).to.be.instanceOf(Uint8Array);
    expect(await wallet.deriveKeyFromSeedPhrase({ mnemonic, passphrase: null, network: 'testnet' })).to.exist();
    expect(await wallet.deriveKeyFromSeedWithPath({
      mnemonic, passphrase: null, path: "m/44'/5'/0'", network: 'testnet',
    })).to.exist();
    expect(await wallet.deriveKeyFromSeedWithExtendedPath({
      mnemonic, passphrase: null, path: "m/15'/0'", network: 'testnet',
    })).to.exist();
  });

  it('key utilities return expected shapes', async () => {
    const kp = await wallet.generateKeyPair('testnet');
    expect(kp).to.be.an('object');
    const kps = await wallet.generateKeyPairs('testnet', 2);
    expect(kps).to.be.an('array');
  });
});
