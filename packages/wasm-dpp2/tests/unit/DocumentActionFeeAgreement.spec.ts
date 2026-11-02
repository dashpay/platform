import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('DocumentActionFeeAgreement', () => {
  describe('constructor()', () => {
    it('should default both amounts to zero and agree to a fixed fee', () => {
      const agreement = new wasm.DocumentActionFeeAgreement({});

      expect(agreement.owner).to.equal(0n);
      expect(agreement.moderators).to.equal(0n);
      expect(agreement.knownFeeMultiplierPermille).to.be.undefined();
      expect(agreement.feeMultiplierIncreaseTolerancePercent).to.be.undefined();
      expect(agreement.pricing).to.equal('fixed');

      agreement.free();
    });

    it('should create an agreement to a fee priced by the fee multiplier', () => {
      const agreement = new wasm.DocumentActionFeeAgreement({
        owner: 10000000n,
        moderators: 100000000n,
        feeMultiplier: {
          knownPermille: 1000n,
          increaseTolerancePercent: 20,
        },
      });

      expect(agreement.owner).to.equal(10000000n);
      expect(agreement.moderators).to.equal(100000000n);
      expect(agreement.knownFeeMultiplierPermille).to.equal(1000n);
      expect(agreement.feeMultiplierIncreaseTolerancePercent).to.equal(20);
      expect(agreement.pricing).to.equal('feeMultiplier');

      agreement.free();
    });

    it('should create an agreement to a fixed fee', () => {
      const agreement = new wasm.DocumentActionFeeAgreement({
        owner: 5000n,
        moderators: 7000n,
      });

      expect(agreement.owner).to.equal(5000n);
      expect(agreement.moderators).to.equal(7000n);
      expect(agreement.knownFeeMultiplierPermille).to.be.undefined();
      expect(agreement.pricing).to.equal('fixed');

      agreement.free();
    });
  });

  describe('forDocumentTypeAction()', () => {
    let contract: InstanceType<typeof wasm.DataContract>;
    const feeMultiplier = { knownPermille: 1000n, increaseTolerancePercent: 20 };

    before(() => {
      const properties = { text: { type: 'string', maxLength: 60, position: 0 } };
      contract = new wasm.DataContract({
        ownerId: '11111111111111111111111111111111',
        identityNonce: 1n,
        schemas: {
          post: {
            type: 'object',
            properties,
            additionalProperties: false,
            actionFees: { create: { owner: 80000000, moderators: 16000000 } },
          },
          note: {
            type: 'object',
            properties,
            additionalProperties: false,
            actionFees: { pricing: 'fixed', create: { owner: 1000 } },
          },
          plain: { type: 'object', properties, additionalProperties: false },
        },
        fullValidation: true,
        platformVersion: new wasm.PlatformVersion(14),
      });
    });

    after(() => {
      contract.free();
    });

    it('should name the declared fee and the multiplier for a fee priced by the multiplier', () => {
      const agreement = wasm.DocumentActionFeeAgreement.forDocumentTypeAction(
        contract,
        'post',
        'create',
        feeMultiplier,
      );

      expect(agreement?.owner).to.equal(80000000n);
      expect(agreement?.moderators).to.equal(16000000n);
      expect(agreement?.pricing).to.equal('feeMultiplier');
      expect(agreement?.knownFeeMultiplierPermille).to.equal(1000n);
      expect(agreement?.feeMultiplierIncreaseTolerancePercent).to.equal(20);
    });

    it('should refuse a fee priced by the multiplier without one', () => {
      expect(() => wasm.DocumentActionFeeAgreement.forDocumentTypeAction(contract, 'post', 'create'))
        .to.throw(/feeMultiplier is required/);
    });

    it('should name no multiplier for a fixed fee, a part left out as 0', () => {
      for (const multiplier of [undefined, feeMultiplier]) {
        const agreement = wasm.DocumentActionFeeAgreement.forDocumentTypeAction(
          contract,
          'note',
          'create',
          multiplier,
        );

        expect(agreement?.owner).to.equal(1000n);
        expect(agreement?.moderators).to.equal(0n);
        expect(agreement?.pricing).to.equal('fixed');
        expect(agreement?.knownFeeMultiplierPermille).to.be.undefined();
      }
    });

    it('should return undefined for an action the type charges nothing for, with no multiplier', () => {
      expect(wasm.DocumentActionFeeAgreement.forDocumentTypeAction(contract, 'post', 'delete'))
        .to.be.undefined();
      expect(wasm.DocumentActionFeeAgreement.forDocumentTypeAction(contract, 'plain', 'create'))
        .to.be.undefined();
    });

    it('should read the action as a BatchType too', () => {
      const agreement = wasm.DocumentActionFeeAgreement.forDocumentTypeAction(
        contract,
        'note',
        wasm.BatchType.Create,
      );

      expect(agreement?.owner).to.equal(1000n);
    });

    it('should refuse an unknown document type or action', () => {
      expect(() => wasm.DocumentActionFeeAgreement.forDocumentTypeAction(contract, 'missing', 'create'))
        .to.throw(/document type not found/);
      expect(() => wasm.DocumentActionFeeAgreement.forDocumentTypeAction(contract, 'post', 'vote' as never))
        .to.throw(/unknown batch type value: vote/);
      [Number.NaN, -1, 0.5].forEach((action) => {
        expect(() => wasm.DocumentActionFeeAgreement.forDocumentTypeAction(contract, 'note', action as never))
          .to.throw(/unknown batch type value/);
      });
    });
  });

  describe('toJSON()', () => {
    it('should serialize with $formatVersion and camelCase fields', () => {
      const agreement = new wasm.DocumentActionFeeAgreement({
        owner: 10000000n,
        moderators: 100000000n,
        feeMultiplier: {
          knownPermille: 1000n,
          increaseTolerancePercent: 20,
        },
      });

      const json = agreement.toJSON();

      expect(json.$formatVersion).to.equal('0');
      expect(json.owner).to.equal(10000000);
      expect(json.moderators).to.equal(100000000);
      expect(json.feeMultiplier).to.deep.equal({
        knownPermille: 1000,
        increaseTolerancePercent: 20,
      });

      agreement.free();
    });

    it('should serialize a null fee multiplier for a fixed fee', () => {
      const agreement = new wasm.DocumentActionFeeAgreement({
        owner: 5000n,
        moderators: 7000n,
      });

      const json = agreement.toJSON();

      expect(json.$formatVersion).to.equal('0');
      expect(json.owner).to.equal(5000);
      expect(json.moderators).to.equal(7000);
      expect(json.feeMultiplier).to.be.null();

      agreement.free();
    });
  });

  describe('fromJSON()', () => {
    it('should round-trip through JSON', () => {
      const agreement = new wasm.DocumentActionFeeAgreement({
        owner: 10000000n,
        moderators: 100000000n,
        feeMultiplier: {
          knownPermille: 1500n,
          increaseTolerancePercent: 5,
        },
      });

      const json = agreement.toJSON();
      const restored = wasm.DocumentActionFeeAgreement.fromJSON(json);

      expect(restored.owner).to.equal(10000000n);
      expect(restored.moderators).to.equal(100000000n);
      expect(restored.knownFeeMultiplierPermille).to.equal(1500n);
      expect(restored.feeMultiplierIncreaseTolerancePercent).to.equal(5);
      expect(restored.pricing).to.equal('feeMultiplier');
      expect(restored.toJSON()).to.deep.equal(json);

      agreement.free();
      restored.free();
    });

    it('should round-trip a fixed fee through JSON', () => {
      const agreement = new wasm.DocumentActionFeeAgreement({
        owner: 5000n,
        moderators: 7000n,
      });

      const json = agreement.toJSON();
      const restored = wasm.DocumentActionFeeAgreement.fromJSON(json);

      expect(restored.owner).to.equal(5000n);
      expect(restored.knownFeeMultiplierPermille).to.be.undefined();
      expect(restored.pricing).to.equal('fixed');
      expect(restored.toJSON()).to.deep.equal(json);

      agreement.free();
      restored.free();
    });
  });

  describe('toObject()', () => {
    it('should serialize with $formatVersion and bigint amounts', () => {
      const agreement = new wasm.DocumentActionFeeAgreement({
        owner: 10000000n,
        moderators: 100000000n,
        feeMultiplier: {
          knownPermille: 1000n,
          increaseTolerancePercent: 20,
        },
      });

      const obj = agreement.toObject();

      expect(obj.$formatVersion).to.equal('0');
      expect(obj.owner).to.equal(10000000n);
      expect(obj.moderators).to.equal(100000000n);
      expect(obj.feeMultiplier.knownPermille).to.equal(1000n);
      expect(obj.feeMultiplier.increaseTolerancePercent).to.equal(20);

      agreement.free();
    });
  });

  describe('fromObject()', () => {
    it('should round-trip through Object', () => {
      const agreement = new wasm.DocumentActionFeeAgreement({
        owner: 10000000n,
        moderators: 100000000n,
        feeMultiplier: {
          knownPermille: 1000n,
          increaseTolerancePercent: 20,
        },
      });

      const obj = agreement.toObject();
      const restored = wasm.DocumentActionFeeAgreement.fromObject(obj);

      expect(restored.owner).to.equal(10000000n);
      expect(restored.moderators).to.equal(100000000n);
      expect(restored.knownFeeMultiplierPermille).to.equal(1000n);
      expect(restored.feeMultiplierIncreaseTolerancePercent).to.equal(20);
      expect(restored.toObject()).to.deep.equal(obj);

      agreement.free();
      restored.free();
    });

    it('should round-trip a fixed fee through Object', () => {
      const agreement = new wasm.DocumentActionFeeAgreement({
        owner: 5000n,
        moderators: 7000n,
      });

      const obj = agreement.toObject();
      const restored = wasm.DocumentActionFeeAgreement.fromObject(obj);

      expect(restored.owner).to.equal(5000n);
      expect(restored.moderators).to.equal(7000n);
      expect(restored.knownFeeMultiplierPermille).to.be.undefined();
      expect(restored.pricing).to.equal('fixed');
      expect(restored.toObject()).to.deep.equal(obj);

      agreement.free();
      restored.free();
    });
  });

  describe('__type', () => {
    it('should return correct __type', () => {
      const agreement = new wasm.DocumentActionFeeAgreement({});
      expect(agreement.__type).to.equal('DocumentActionFeeAgreement');
      agreement.free();
    });
  });
});
