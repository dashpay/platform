import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

describe('StateTransitionProofResult types', () => {
  const testIdentifier = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

  before(async () => {
    await init();
  });

  const variantClasses = [
    'VerifiedDataContract',
    'VerifiedIdentity',
    'VerifiedTokenBalanceAbsence',
    'VerifiedTokenBalance',
    'VerifiedTokenIdentityInfo',
    'VerifiedTokenPricingSchedule',
    'VerifiedTokenStatus',
    'VerifiedTokenIdentitiesBalances',
    'VerifiedPartialIdentity',
    'VerifiedBalanceTransfer',
    'VerifiedDocuments',
    'VerifiedTokenActionWithDocument',
    'VerifiedTokenGroupActionWithDocument',
    'VerifiedTokenGroupActionWithTokenBalance',
    'VerifiedTokenGroupActionWithTokenIdentityInfo',
    'VerifiedTokenGroupActionWithTokenPricingSchedule',
    'VerifiedMasternodeVote',
    'VerifiedNextDistribution',
    'VerifiedAddressInfos',
    'VerifiedIdentityFullWithAddressInfos',
    'VerifiedIdentityWithAddressInfos',
  ];

  describe('exports', () => {
    for (const name of variantClasses) {
      it(`should export ${name} class`, () => {
        expect(sdk[name]).to.be.a('function');
      });
    }
  });

  describe('__struct getter', () => {
    for (const name of variantClasses) {
      it(`${name}.__struct should return "${name}"`, () => {
        expect(sdk[name].__struct).to.equal(name);
      });
    }
  });

  describe('conversion methods', () => {
    for (const name of variantClasses) {
      it(`${name} should have toObject, fromObject, toJSON, fromJSON`, () => {
        expect(typeof sdk[name].fromObject).to.equal('function');
        expect(typeof sdk[name].fromJSON).to.equal('function');
        expect(typeof sdk[name].prototype.toObject).to.equal('function');
        expect(typeof sdk[name].prototype.toJSON).to.equal('function');
      });
    }
  });

  describe('VerifiedTokenBalanceAbsence', () => {
    it('should round-trip serializable fields through fromObject/toObject', () => {
      const data = { tokenId: testIdentifier };
      const result = sdk.VerifiedTokenBalanceAbsence.fromObject(data);

      expect(result.tokenId.toBase58()).to.equal(testIdentifier);

      const obj = result.toObject();
      expect(obj.tokenId).to.equal(testIdentifier);

      const roundtrip = sdk.VerifiedTokenBalanceAbsence.fromObject(obj);
      expect(roundtrip.tokenId.toBase58()).to.equal(testIdentifier);
    });

    it('should round-trip through fromJSON/toJSON', () => {
      const data = { tokenId: testIdentifier };
      const result = sdk.VerifiedTokenBalanceAbsence.fromJSON(data);

      const json = result.toJSON();
      expect(json.tokenId).to.equal(testIdentifier);

      const roundtrip = sdk.VerifiedTokenBalanceAbsence.fromJSON(json);
      expect(roundtrip.tokenId.toBase58()).to.equal(testIdentifier);
    });
  });

  describe('VerifiedTokenBalance', () => {
    it('should round-trip serializable fields through fromObject/toObject', () => {
      const data = { tokenId: testIdentifier, balance: 500000n };
      const result = sdk.VerifiedTokenBalance.fromObject(data);

      expect(result.tokenId.toBase58()).to.equal(testIdentifier);
      expect(result.balance).to.equal(500000n);

      const obj = result.toObject();
      expect(obj.tokenId).to.equal(testIdentifier);
      expect(obj.balance).to.equal(500000n);
    });
  });

  describe('VerifiedTokenGroupActionWithTokenBalance', () => {
    it('should round-trip serializable fields through fromObject/toObject', () => {
      const data = {
        groupPower: 42,
        actionStatus: 'ActionActive',
        balance: 100000n,
      };
      const result = sdk.VerifiedTokenGroupActionWithTokenBalance.fromObject(data);

      expect(result.groupPower).to.equal(42);
      expect(result.actionStatus).to.equal('ActionActive');
      expect(result.balance).to.equal(100000n);

      const obj = result.toObject();
      expect(obj.groupPower).to.equal(42);
      expect(obj.actionStatus).to.equal('ActionActive');

      const roundtrip = sdk.VerifiedTokenGroupActionWithTokenBalance.fromObject(obj);
      expect(roundtrip.groupPower).to.equal(42);
      expect(roundtrip.actionStatus).to.equal('ActionActive');
    });
  });

  describe('VerifiedDataContract (all fields skipped)', () => {
    it('should create from empty object (all fields are serde-skipped)', () => {
      const result = sdk.VerifiedDataContract.fromObject({});
      expect(result.dataContract).to.be.undefined();
    });

    it('should produce empty object from toObject', () => {
      const result = sdk.VerifiedDataContract.fromObject({});
      const obj = result.toObject();
      expect(obj).to.be.an('object');
    });
  });

  describe('VerifiedTokenGroupActionWithDocument', () => {
    it('should serialize groupPower but skip document', () => {
      const data = { groupPower: 10 };
      const result = sdk.VerifiedTokenGroupActionWithDocument.fromObject(data);

      expect(result.groupPower).to.equal(10);
      expect(result.document).to.be.undefined();

      const obj = result.toObject();
      expect(obj.groupPower).to.equal(10);
    });
  });
});
