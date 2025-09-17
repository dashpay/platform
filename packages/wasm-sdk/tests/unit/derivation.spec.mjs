import init, * as sdk from '../../dist/sdk.js';

describe('Key derivation', () => {
  before(async () => {
    await init();
  });

  describe('Path helpers (BIP44/DIP9/DIP13)', () => {
    it('BIP44 mainnet/testnet', () => {
      const m = sdk.WasmSdk.derivationPathBip44Mainnet(0, 0, 0);
      expect(m.purpose).to.equal(44);
      expect(m.coin_type).to.equal(5);
      expect(m.account).to.equal(0);
      expect(m.change).to.equal(0);
      expect(m.index).to.equal(0);
      const expectedMain = `m/${m.purpose}'/${m.coin_type}'/${m.account}'/${m.change}/${m.index}`;
      expect(expectedMain).to.equal("m/44'/5'/0'/0/0");

      const t = sdk.WasmSdk.derivationPathBip44Testnet(0, 0, 0);
      expect(t.coin_type).to.equal(1);
      const expectedTest = `m/${t.purpose}'/${t.coin_type}'/${t.account}'/${t.change}/${t.index}`;
      expect(expectedTest).to.equal("m/44'/1'/0'/0/0");
    });

    it('DIP9 mainnet/testnet', () => {
      const m = sdk.WasmSdk.derivationPathDip9Mainnet(5, 0, 0);
      expect(m.purpose).to.equal(9);
      expect(m.coin_type).to.equal(5);
      expect(m.account).to.equal(5);
      const expectedMain = `m/${m.purpose}'/${m.coin_type}'/${m.account}'/${m.change}/${m.index}`;
      expect(expectedMain).to.equal("m/9'/5'/5'/0/0");

      const t = sdk.WasmSdk.derivationPathDip9Testnet(5, 0, 0);
      expect(t.coin_type).to.equal(1);
      const expectedTest = `m/${t.purpose}'/${t.coin_type}'/${t.account}'/${t.change}/${t.index}`;
      expect(expectedTest).to.equal("m/9'/1'/5'/0/0");
    });

    it('DIP13 mainnet/testnet', () => {
      const m = sdk.WasmSdk.derivationPathDip13Mainnet(0);
      expect(m.path).to.equal("m/9'/5'/0'");
      expect(m.purpose).to.equal(9);
      expect(m.description).to.equal('DIP13 HD identity key path');

      const t = sdk.WasmSdk.derivationPathDip13Testnet(0);
      expect(t.path).to.equal("m/9'/1'/0'");
    });
  });

  describe('Derive by path', () => {
    const seed = 'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about';

    it('BIP44 mainnet key', () => {
      const path = "m/44'/5'/0'/0/0";
      const r = sdk.WasmSdk.deriveKeyFromSeedWithPath(seed, undefined, path, 'mainnet');
      expect(r).to.exist();
      expect(r.path).to.equal(path);
      expect(r.address.startsWith('X')).to.equal(true);
      expect(r.network).to.equal('mainnet');
    });

    it('DIP13 authentication key', () => {
      const path = "m/9'/5'/5'/0'/0'/0'/0'";
      const r = sdk.WasmSdk.deriveKeyFromSeedWithPath(seed, undefined, path, 'mainnet');
      expect(r).to.exist();
      expect(r.path).to.equal(path);
      expect(r.private_key_wif).to.be.a('string');
      expect(r.address).to.be.a('string');
    });

    it('with passphrase produces different address', () => {
      const path = "m/44'/5'/0'/0/0";
      const withPass = sdk.WasmSdk.deriveKeyFromSeedWithPath(seed, 'test passphrase', path, 'mainnet');
      const withoutPass = sdk.WasmSdk.deriveKeyFromSeedWithPath(seed, undefined, path, 'mainnet');
      expect(withPass.address).to.not.equal(withoutPass.address);
    });

    it('testnet address prefix', () => {
      const path = "m/44'/1'/0'/0/0";
      const r = sdk.WasmSdk.deriveKeyFromSeedWithPath(seed, undefined, path, 'testnet');
      expect(r.network).to.equal('testnet');
      expect(r.address.startsWith('y')).to.equal(true);
    });

    it('DIP9 hardened vs non-hardened differ', () => {
      const hardened = sdk.WasmSdk.deriveKeyFromSeedWithPath(seed, null, "m/9'/5'/5'/0/0", 'mainnet');
      const nonHardened = sdk.WasmSdk.deriveKeyFromSeedWithPath(seed, null, 'm/9/5/5/0/0', 'mainnet');
      expect(hardened.address).to.not.equal(nonHardened.address);
    });
  });

  describe('DIP14 extended vectors', () => {
    const mnemonic = 'birth kingdom trash renew flavor utility donkey gasp regular alert pave layer';

    it('Vector 1: mixed hardened/non-hardened', () => {
      const path = "m/0x775d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3b/0xf537439f36d04a15474ff7423e4b904a14373fafb37a41db74c84f1dbb5c89a6'/0x4c4592ca670c983fc43397dfd21a6f427fac9b4ac53cb4dcdc6522ec51e81e79/0";
      const r = sdk.WasmSdk.deriveKeyFromSeedWithExtendedPath(mnemonic, null, path, 'testnet');
      expect(r.xprv).to.be.a('string');
      expect(r.xpub).to.be.a('string');
    });

    it('Vector 2: multiple hardened with final non-hardened', () => {
      const path = "m/9'/5'/15'/0'/0x555d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3a'/0xa137439f36d04a15474ff7423e4b904a14373fafb37a41db74c84f1dbb5c89b5'/0";
      const r = sdk.WasmSdk.deriveKeyFromSeedWithExtendedPath(mnemonic, null, path, 'testnet');
      expect(r.xprv).to.be.a('string');
      expect(r.xpub).to.be.a('string');
    });
  });

  describe('DIP15 DashPay contact keys', () => {
    const mnemonic = 'birth kingdom trash renew flavor utility donkey gasp regular alert pave layer';
    const sender = '0x555d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3a';
    const receiver = '0xa137439f36d04a15474ff7423e4b904a14373fafb37a41db74c84f1dbb5c89b5';

    it('deterministic contact key for testnet', () => {
      const r1 = sdk.WasmSdk.deriveDashpayContactKey(mnemonic, null, sender, receiver, 0, 0, 'testnet');
      const r2 = sdk.WasmSdk.deriveDashpayContactKey(mnemonic, null, sender, receiver, 0, 0, 'testnet');

      expect(r1).to.be.ok();
      expect(r1).to.have.property('path');
      expect(r1).to.have.property('xprv');
      expect(r1).to.have.property('xpub');
      expect(r1).to.have.property('private_key_hex');
      expect(r1.private_key_hex).to.have.length(64);

      expect(r2.private_key_hex).to.equal(r1.private_key_hex);
      expect(r2.xprv).to.equal(r1.xprv);
      expect(r2.xpub).to.equal(r1.xpub);

      expect(r1.path).to.include("15'");
      expect(r1.path).to.include(sender);
      expect(r1.path).to.include(receiver);

      expect(r1.xprv.startsWith('tprv')).to.equal(true);
      expect(r1.xpub.startsWith('tpub')).to.equal(true);
    });

    it('changes when sender/receiver are swapped', () => {
      const a = sdk.WasmSdk.deriveDashpayContactKey(mnemonic, null, sender, receiver, 0, 0, 'testnet');
      const b = sdk.WasmSdk.deriveDashpayContactKey(mnemonic, null, receiver, sender, 0, 0, 'testnet');
      expect(a.private_key_hex).to.not.equal(b.private_key_hex);
    });

    it('differs between networks (testnet vs mainnet)', () => {
      const t = sdk.WasmSdk.deriveDashpayContactKey(mnemonic, null, sender, receiver, 0, 0, 'testnet');
      const m = sdk.WasmSdk.deriveDashpayContactKey(mnemonic, null, sender, receiver, 0, 0, 'mainnet');
      expect(m.xprv.startsWith('xprv')).to.equal(true);
      expect(m.xpub.startsWith('xpub')).to.equal(true);
      expect(m.private_key_hex).to.not.equal(t.private_key_hex);
    });
  });
});
