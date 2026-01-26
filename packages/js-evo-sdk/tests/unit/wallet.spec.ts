import { expect } from 'chai';
import { wallet } from '../../dist/sdk.js';

describe('wallet', () => {
  const exportedFns = [
    'generateMnemonic',
    'validateMnemonic',
    'mnemonicToSeed',
    'deriveKeyFromSeedPhrase',
    'deriveKeyFromSeedWithPath',
    'deriveKeyFromSeedWithExtendedPath',
    'deriveDashpayContactKey',
    'derivationPathBip44Mainnet',
    'derivationPathBip44Testnet',
    'derivationPathDip9Mainnet',
    'derivationPathDip9Testnet',
    'derivationPathDip13Mainnet',
    'derivationPathDip13Testnet',
    'deriveChildPublicKey',
    'xprvToXpub',
    'generateKeyPair',
    'generateKeyPairs',
    'keyPairFromWif',
    'keyPairFromHex',
    'pubkeyToAddress',
    'validateAddress',
    'signMessage',
  ];

  describe('exports', () => {
    it('should expose the expected helper functions', () => {
      exportedFns.forEach((fn) => {
        expect(wallet).to.have.property(fn).that.is.a('function');
      });
    });
  });
});
