import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

describe('Derivation Type Conversions', () => {
  before(async () => {
    await init();
  });

  describe('DerivationPathInfo', () => {
    const jsonFixture = {
      path: "m/44'/5'/0'/0/0",
      purpose: 44,
      coinType: 5,
      account: 0,
      change: 0,
      index: 0,
    };

    describe('toJSON()', () => {
      it('should produce expected JSON', () => {
        const result = sdk.DerivationPathInfo.fromJSON(jsonFixture);
        expect(result.toJSON()).to.deep.equal(jsonFixture);
      });
    });

    describe('toObject()', () => {
      it('should produce expected Object', () => {
        const result = sdk.DerivationPathInfo.fromJSON(jsonFixture);
        const obj = result.toObject();
        const obj2 = sdk.DerivationPathInfo.fromObject(obj).toObject();
        expect(obj2).to.deep.equal(obj);
      });
    });

    describe('fromJSON()', () => {
      it('should deserialize and expose getters', () => {
        const result = sdk.DerivationPathInfo.fromJSON(jsonFixture);
        expect(result.path).to.equal("m/44'/5'/0'/0/0");
        expect(result.purpose).to.equal(44);
        expect(result.coinType).to.equal(5);
        expect(result.account).to.equal(0);
        expect(result.change).to.equal(0);
        expect(result.index).to.equal(0);
      });
    });
  });

  describe('Dip13DerivationPathInfo', () => {
    const jsonFixture = {
      path: "m/9'/5'/15'/0'",
      purpose: 9,
      coinType: 5,
      account: 0,
      description: 'DIP13 identity authentication key',
    };

    describe('toJSON()', () => {
      it('should produce expected JSON', () => {
        const result = sdk.Dip13DerivationPathInfo.fromJSON(jsonFixture);
        expect(result.toJSON()).to.deep.equal(jsonFixture);
      });
    });

    describe('toObject()', () => {
      it('should produce expected Object', () => {
        const result = sdk.Dip13DerivationPathInfo.fromJSON(jsonFixture);
        const obj = result.toObject();
        const obj2 = sdk.Dip13DerivationPathInfo.fromObject(obj).toObject();
        expect(obj2).to.deep.equal(obj);
      });
    });

    describe('fromJSON()', () => {
      it('should deserialize and expose getters', () => {
        const result = sdk.Dip13DerivationPathInfo.fromJSON(jsonFixture);
        expect(result.path).to.equal("m/9'/5'/15'/0'");
        expect(result.purpose).to.equal(9);
        expect(result.coinType).to.equal(5);
        expect(result.account).to.equal(0);
        expect(result.description).to.equal('DIP13 identity authentication key');
      });
    });
  });

  describe('SeedPhraseKeyInfo', () => {
    const jsonFixture = {
      privateKeyWif: 'cNYPkC4hGoE11bMkiCbfvb55ygXdBhazLkEhcEndxM8bufmXQ5ZZ',
      privateKeyHex: '1234abcd5678ef90',
      publicKey: '02eab222e32d46b97f56fb90bb22c3d65e279b18bda203f30bd2d3eed769a34762',
      address: 'yNPbcFfabtNmmxKdGwhHomdYfVs6gikbPf',
      network: 'testnet',
    };

    describe('toJSON()', () => {
      it('should produce expected JSON', () => {
        const result = sdk.SeedPhraseKeyInfo.fromJSON(jsonFixture);
        expect(result.toJSON()).to.deep.equal(jsonFixture);
      });
    });

    describe('toObject()', () => {
      it('should produce expected Object', () => {
        const result = sdk.SeedPhraseKeyInfo.fromJSON(jsonFixture);
        const obj = result.toObject();
        const obj2 = sdk.SeedPhraseKeyInfo.fromObject(obj).toObject();
        expect(obj2).to.deep.equal(obj);
      });
    });

    describe('fromJSON()', () => {
      it('should deserialize and expose getters', () => {
        const result = sdk.SeedPhraseKeyInfo.fromJSON(jsonFixture);
        expect(result.privateKeyWif).to.equal(jsonFixture.privateKeyWif);
        expect(result.publicKey).to.equal(jsonFixture.publicKey);
        expect(result.address).to.equal(jsonFixture.address);
        expect(result.network).to.equal('testnet');
      });
    });
  });

  describe('PathDerivedKeyInfo', () => {
    const jsonFixture = {
      path: "m/44'/5'/0'/0/0",
      privateKeyWif: 'cNYPkC4hGoE11bMkiCbfvb55ygXdBhazLkEhcEndxM8bufmXQ5ZZ',
      privateKeyHex: '1234abcd5678ef90',
      publicKey: '02eab222e32d46b97f56fb90bb22c3d65e279b18bda203f30bd2d3eed769a34762',
      address: 'yNPbcFfabtNmmxKdGwhHomdYfVs6gikbPf',
      network: 'testnet',
    };

    describe('toJSON()', () => {
      it('should produce expected JSON', () => {
        const result = sdk.PathDerivedKeyInfo.fromJSON(jsonFixture);
        expect(result.toJSON()).to.deep.equal(jsonFixture);
      });
    });

    describe('toObject()', () => {
      it('should produce expected Object', () => {
        const result = sdk.PathDerivedKeyInfo.fromJSON(jsonFixture);
        const obj = result.toObject();
        const obj2 = sdk.PathDerivedKeyInfo.fromObject(obj).toObject();
        expect(obj2).to.deep.equal(obj);
      });
    });

    describe('fromJSON()', () => {
      it('should deserialize and expose getters', () => {
        const result = sdk.PathDerivedKeyInfo.fromJSON(jsonFixture);
        expect(result.path).to.equal("m/44'/5'/0'/0/0");
        expect(result.address).to.equal(jsonFixture.address);
        expect(result.network).to.equal('testnet');
      });
    });
  });

  describe('DerivedKeyInfo', () => {
    const jsonFixture = {
      path: "m/44'/5'/0'/0/0",
      privateKeyWif: 'cNYPkC4hGoE11bMkiCbfvb55ygXdBhazLkEhcEndxM8bufmXQ5ZZ',
      privateKeyHex: '1234abcd',
      publicKey: '02eab222e32d46b97f56fb90',
      address: 'yNPbcFfabtNmmxKdGwhHomdYfVs6gikbPf',
      network: 'testnet',
      xprv: 'tprv8ZgxMBicQKsPdTest',
      xpub: 'tpubD6NzVbkrYhZ4Test',
    };

    describe('toJSON()', () => {
      it('should produce expected JSON', () => {
        const result = sdk.DerivedKeyInfo.fromJSON(jsonFixture);
        expect(result.toJSON()).to.deep.equal(jsonFixture);
      });
    });

    describe('toObject()', () => {
      it('should produce expected Object', () => {
        const result = sdk.DerivedKeyInfo.fromJSON(jsonFixture);
        const obj = result.toObject();
        const obj2 = sdk.DerivedKeyInfo.fromObject(obj).toObject();
        expect(obj2).to.deep.equal(obj);
      });
    });

    describe('fromJSON()', () => {
      it('should deserialize and expose getters', () => {
        const result = sdk.DerivedKeyInfo.fromJSON(jsonFixture);
        expect(result.path).to.equal("m/44'/5'/0'/0/0");
        expect(result.address).to.equal(jsonFixture.address);
        expect(result.xprv).to.equal(jsonFixture.xprv);
        expect(result.xpub).to.equal(jsonFixture.xpub);
      });
    });
  });

  describe('DashpayContactKeyInfo', () => {
    const jsonFixture = {
      path: "m/15'/5'/0'/0/0",
      privateKeyWif: 'cNYPkC4hGoE11bMkiCbfvb55ygXdBhazLkEhcEndxM8bufmXQ5ZZ',
      privateKeyHex: '1234abcd',
      publicKey: '02eab222e32d46b97f56fb90',
      address: 'yNPbcFfabtNmmxKdGwhHomdYfVs6gikbPf',
      network: 'testnet',
      xprv: 'tprv8ZgxMBicQKsPdTest',
      xpub: 'tpubD6NzVbkrYhZ4Test',
      dipStandard: 'DIP15',
      purpose: 'DashPay Contact Payment',
      senderIdentity: 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1',
      receiverIdentity: '2QjL594djCH2NyDsn45vd6yQjEDHupMKo7CEGVTHtQxU',
      account: 0,
      addressIndex: 0,
    };

    describe('toJSON()', () => {
      it('should produce expected JSON', () => {
        const result = sdk.DashpayContactKeyInfo.fromJSON(jsonFixture);
        expect(result.toJSON()).to.deep.equal(jsonFixture);
      });
    });

    describe('toObject()', () => {
      it('should produce expected Object', () => {
        const result = sdk.DashpayContactKeyInfo.fromJSON(jsonFixture);
        const obj = result.toObject();
        const obj2 = sdk.DashpayContactKeyInfo.fromObject(obj).toObject();
        expect(obj2).to.deep.equal(obj);
      });
    });

    describe('fromJSON()', () => {
      it('should deserialize and expose getters', () => {
        const result = sdk.DashpayContactKeyInfo.fromJSON(jsonFixture);
        expect(result.dipStandard).to.equal('DIP15');
        expect(result.purpose).to.equal('DashPay Contact Payment');
        expect(result.senderIdentity).to.equal(jsonFixture.senderIdentity);
        expect(result.receiverIdentity).to.equal(jsonFixture.receiverIdentity);
        expect(result.account).to.equal(0);
        expect(result.addressIndex).to.equal(0);
      });
    });
  });

  describe('KeyPair', () => {
    describe('toJSON()', () => {
      it('should round-trip with stable output', () => {
        const kp = sdk.WasmSdk.generateKeyPair('testnet');
        const json = kp.toJSON();
        const json2 = sdk.KeyPair.fromJSON(json).toJSON();
        expect(json2).to.deep.equal(json);
      });
    });

    describe('toObject()', () => {
      it('should round-trip with stable output', () => {
        const kp = sdk.WasmSdk.generateKeyPair('testnet');
        const obj = kp.toObject();
        const obj2 = sdk.KeyPair.fromObject(obj).toObject();
        expect(obj2).to.deep.equal(obj);
      });
    });
  });
});
