import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

describe('Key Derivation', () => {
  before(async () => {
    await init();
  });

  describe('derivationPathBip44Mainnet()', () => {
    it('should derive BIP44 mainnet paths', () => {
      const m = sdk.WasmSdk.derivationPathBip44Mainnet(0, 0, 0);
      expect(m.purpose).to.equal(44);
      expect(m.coinType).to.equal(5);
      expect(m.account).to.equal(0);
      expect(m.change).to.equal(0);
      expect(m.index).to.equal(0);
      const expectedMain = `m/${m.purpose}'/${m.coinType}'/${m.account}'/${m.change}/${m.index}`;
      expect(expectedMain).to.equal("m/44'/5'/0'/0/0");
    });
  });

  describe('derivationPathBip44Testnet()', () => {
    it('should derive BIP44 testnet paths', () => {
      const t = sdk.WasmSdk.derivationPathBip44Testnet(0, 0, 0);
      expect(t.coinType).to.equal(1);
      const expectedTest = `m/${t.purpose}'/${t.coinType}'/${t.account}'/${t.change}/${t.index}`;
      expect(expectedTest).to.equal("m/44'/1'/0'/0/0");
    });
  });

  describe('derivationPathDip9Mainnet()', () => {
    it('should derive DIP9 mainnet paths', () => {
      const m = sdk.WasmSdk.derivationPathDip9Mainnet(5, 0, 0);
      expect(m.purpose).to.equal(9);
      expect(m.coinType).to.equal(5);
      expect(m.account).to.equal(5);
      const expectedMain = `m/${m.purpose}'/${m.coinType}'/${m.account}'/${m.change}/${m.index}`;
      expect(expectedMain).to.equal("m/9'/5'/5'/0/0");
    });
  });

  describe('derivationPathDip9Testnet()', () => {
    it('should derive DIP9 testnet paths', () => {
      const t = sdk.WasmSdk.derivationPathDip9Testnet(5, 0, 0);
      expect(t.coinType).to.equal(1);
      const expectedTest = `m/${t.purpose}'/${t.coinType}'/${t.account}'/${t.change}/${t.index}`;
      expect(expectedTest).to.equal("m/9'/1'/5'/0/0");
    });
  });

  describe('derivationPathDip13Mainnet()', () => {
    it('should derive DIP13 mainnet paths', () => {
      const m = sdk.WasmSdk.derivationPathDip13Mainnet(0);
      expect(m.path).to.equal("m/9'/5'/0'");
      expect(m.purpose).to.equal(9);
      expect(m.description).to.equal('DIP13 HD identity key path');
    });
  });

  describe('derivationPathDip13Testnet()', () => {
    it('should derive DIP13 testnet paths', () => {
      const t = sdk.WasmSdk.derivationPathDip13Testnet(0);
      expect(t.path).to.equal("m/9'/1'/0'");
    });
  });

  describe('deriveKeyFromSeedWithPath()', () => {
    const seed = 'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about';

    it('should derive BIP44 mainnet key', () => {
      const path = "m/44'/5'/0'/0/0";
      const r = sdk.WasmSdk.deriveKeyFromSeedWithPath({
        mnemonic: seed, passphrase: null, path, network: 'mainnet',
      });
      expect(r).to.exist();
      expect(r.path).to.equal(path);
      expect(r.address.startsWith('X')).to.equal(true);
      expect(r.network).to.equal('mainnet');
    });

    it('should derive DIP13 authentication key', () => {
      const path = "m/9'/5'/5'/0'/0'/0'/0'";
      const r = sdk.WasmSdk.deriveKeyFromSeedWithPath({
        mnemonic: seed, passphrase: null, path, network: 'mainnet',
      });
      expect(r).to.exist();
      expect(r.path).to.equal(path);
      expect(r.privateKeyWif).to.be.a('string');
      expect(r.address).to.be.a('string');
    });

    it('should produce different address with passphrase', () => {
      const path = "m/44'/5'/0'/0/0";
      const withPass = sdk.WasmSdk.deriveKeyFromSeedWithPath({
        mnemonic: seed, passphrase: 'test passphrase', path, network: 'mainnet',
      });
      const withoutPass = sdk.WasmSdk.deriveKeyFromSeedWithPath({
        mnemonic: seed, passphrase: null, path, network: 'mainnet',
      });
      expect(withPass.address).to.not.equal(withoutPass.address);
    });

    it('should use correct testnet address prefix', () => {
      const path = "m/44'/1'/0'/0/0";
      const r = sdk.WasmSdk.deriveKeyFromSeedWithPath({
        mnemonic: seed, passphrase: null, path, network: 'testnet',
      });
      expect(r.network).to.equal('testnet');
      expect(r.address.startsWith('y')).to.equal(true);
    });

    it('should produce different keys for DIP9 hardened vs non-hardened', () => {
      const hardened = sdk.WasmSdk.deriveKeyFromSeedWithPath({
        mnemonic: seed, passphrase: null, path: "m/9'/5'/5'/0/0", network: 'mainnet',
      });
      const nonHardened = sdk.WasmSdk.deriveKeyFromSeedWithPath({
        mnemonic: seed, passphrase: null, path: 'm/9/5/5/0/0', network: 'mainnet',
      });
      expect(hardened.address).to.not.equal(nonHardened.address);
    });
  });

  describe('deriveKeyFromSeedWithExtendedPath()', () => {
    const mnemonic = 'birth kingdom trash renew flavor utility donkey gasp regular alert pave layer';

    it('should derive DIP14 Vector 1: mixed hardened/non-hardened', () => {
      const path = 'm/0x775d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3b'
        + "/0xf537439f36d04a15474ff7423e4b904a14373fafb37a41db74c84f1dbb5c89a6'"
        + '/0x4c4592ca670c983fc43397dfd21a6f427fac9b4ac53cb4dcdc6522ec51e81e79/0';
      const r = sdk.WasmSdk.deriveKeyFromSeedWithExtendedPath({
        mnemonic, passphrase: null, path, network: 'testnet',
      });
      expect(r.xprv).to.be.a('string');
      expect(r.xpub).to.be.a('string');
    });

    it('should derive DIP14 Vector 2: multiple hardened with final non-hardened', () => {
      const path = "m/9'/5'/15'/0'"
        + "/0x555d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3a'"
        + "/0xa137439f36d04a15474ff7423e4b904a14373fafb37a41db74c84f1dbb5c89b5'/0";
      const r = sdk.WasmSdk.deriveKeyFromSeedWithExtendedPath({
        mnemonic, passphrase: null, path, network: 'testnet',
      });
      expect(r.xprv).to.be.a('string');
      expect(r.xpub).to.be.a('string');
    });
  });

  describe('deriveDashpayContactKey()', () => {
    const mnemonic = 'birth kingdom trash renew flavor utility donkey gasp regular alert pave layer';
    // Hex without 0x prefix (we don't use 0x)
    const sender = '555d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3a';
    const receiver = 'a137439f36d04a15474ff7423e4b904a14373fafb37a41db74c84f1dbb5c89b5';

    it('should derive deterministic DIP15 contact key for testnet', () => {
      const r1 = sdk.WasmSdk.deriveDashpayContactKey({
        mnemonic, passphrase: null, senderIdentityId: sender, receiverIdentityId: receiver, account: 0, addressIndex: 0, network: 'testnet',
      });
      const r2 = sdk.WasmSdk.deriveDashpayContactKey({
        mnemonic, passphrase: null, senderIdentityId: sender, receiverIdentityId: receiver, account: 0, addressIndex: 0, network: 'testnet',
      });

      expect(r1).to.be.ok();
      expect(r1).to.have.property('path');
      expect(r1).to.have.property('xprv');
      expect(r1).to.have.property('xpub');
      expect(r1).to.have.property('privateKeyHex');
      expect(r1.privateKeyHex).to.have.length(64);

      expect(r2.privateKeyHex).to.equal(r1.privateKeyHex);
      expect(r2.xprv).to.equal(r1.xprv);
      expect(r2.xpub).to.equal(r1.xpub);

      expect(r1.path).to.include("15'");
      expect(r1.path).to.include(sender);
      expect(r1.path).to.include(receiver);

      expect(r1.xprv.startsWith('tprv')).to.equal(true);
      expect(r1.xpub.startsWith('tpub')).to.equal(true);
    });

    it('should change DIP15 contact key when sender/receiver are swapped', () => {
      const a = sdk.WasmSdk.deriveDashpayContactKey({
        mnemonic, passphrase: null, senderIdentityId: sender, receiverIdentityId: receiver, account: 0, addressIndex: 0, network: 'testnet',
      });
      const b = sdk.WasmSdk.deriveDashpayContactKey({
        mnemonic, passphrase: null, senderIdentityId: receiver, receiverIdentityId: sender, account: 0, addressIndex: 0, network: 'testnet',
      });
      expect(a.privateKeyHex).to.not.equal(b.privateKeyHex);
    });

    it('should differ DIP15 contact key between networks (testnet vs mainnet)', () => {
      const t = sdk.WasmSdk.deriveDashpayContactKey({
        mnemonic, passphrase: null, senderIdentityId: sender, receiverIdentityId: receiver, account: 0, addressIndex: 0, network: 'testnet',
      });
      const m = sdk.WasmSdk.deriveDashpayContactKey({
        mnemonic, passphrase: null, senderIdentityId: sender, receiverIdentityId: receiver, account: 0, addressIndex: 0, network: 'mainnet',
      });
      expect(m.xprv.startsWith('xprv')).to.equal(true);
      expect(m.xpub.startsWith('xpub')).to.equal(true);
      expect(m.privateKeyHex).to.not.equal(t.privateKeyHex);
    });
  });
});
