import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('ResourceVoteChoice', () => {
  const identityIdHex = '1111111111111111111111111111111111111111111111111111111111111111';

  describe('TowardsIdentity()', () => {
    it('should create TowardsIdentity choice', () => {
      const identityId = wasm.Identifier.fromBytes(Buffer.from(identityIdHex, 'hex'));
      const choice = wasm.ResourceVoteChoice.TowardsIdentity(identityId);

      expect(choice.voteType).to.equal('TowardsIdentity');
      expect(choice.value).to.not.be.undefined();
    });
  });

  describe('Abstain()', () => {
    it('should create Abstain choice', () => {
      const choice = wasm.ResourceVoteChoice.Abstain();

      expect(choice.voteType).to.equal('Abstain');
      expect(choice.value).to.be.undefined();
    });
  });

  describe('Lock()', () => {
    it('should create Lock choice', () => {
      const choice = wasm.ResourceVoteChoice.Lock();

      expect(choice.voteType).to.equal('Lock');
      expect(choice.value).to.be.undefined();
    });
  });

  describe('__type', () => {
    it('should return correct __type', () => {
      const choice = wasm.ResourceVoteChoice.Abstain();

      expect(choice.__type).to.equal('ResourceVoteChoice');
    });
  });

  describe('voteType', () => {
    it('should return TowardsIdentity for TowardsIdentity choice', () => {
      const identityId = wasm.Identifier.fromBytes(Buffer.from(identityIdHex, 'hex'));
      const choice = wasm.ResourceVoteChoice.TowardsIdentity(identityId);

      expect(choice.voteType).to.equal('TowardsIdentity');
    });

    it('should return Abstain for Abstain choice', () => {
      const choice = wasm.ResourceVoteChoice.Abstain();

      expect(choice.voteType).to.equal('Abstain');
    });

    it('should return Lock for Lock choice', () => {
      const choice = wasm.ResourceVoteChoice.Lock();

      expect(choice.voteType).to.equal('Lock');
    });
  });

  describe('toJSON()', () => {
    it('should serialize TowardsIdentity to JSON object', () => {
      const identityId = wasm.Identifier.fromBytes(Buffer.from(identityIdHex, 'hex'));
      const choice = wasm.ResourceVoteChoice.TowardsIdentity(identityId);

      const json = choice.toJSON();
      expect(json).to.be.an('object');
    });

    it('should serialize Abstain to JSON string', () => {
      const choice = wasm.ResourceVoteChoice.Abstain();

      const json = choice.toJSON();
      // Simple enum variants serialize to strings in serde
      expect(json).to.equal('abstain');
    });

    it('should serialize Lock to JSON string', () => {
      const choice = wasm.ResourceVoteChoice.Lock();

      const json = choice.toJSON();
      // Simple enum variants serialize to strings in serde
      expect(json).to.equal('lock');
    });
  });

  describe('fromJSON()', () => {
    it('should deserialize TowardsIdentity from JSON', () => {
      const identityId = wasm.Identifier.fromBytes(Buffer.from(identityIdHex, 'hex'));
      const choice = wasm.ResourceVoteChoice.TowardsIdentity(identityId);

      const json = choice.toJSON();
      const restored = wasm.ResourceVoteChoice.fromJSON(json);
      expect(restored.voteType).to.equal(choice.voteType);
    });

    it('should deserialize Abstain from JSON', () => {
      const choice = wasm.ResourceVoteChoice.Abstain();

      const json = choice.toJSON();
      const restored = wasm.ResourceVoteChoice.fromJSON(json);
      expect(restored.voteType).to.equal('Abstain');
    });

    it('should deserialize Lock from JSON', () => {
      const choice = wasm.ResourceVoteChoice.Lock();

      const json = choice.toJSON();
      const restored = wasm.ResourceVoteChoice.fromJSON(json);
      expect(restored.voteType).to.equal('Lock');
    });
  });

  describe('toObject()', () => {
    it('should serialize TowardsIdentity to object', () => {
      const identityId = wasm.Identifier.fromBytes(Buffer.from(identityIdHex, 'hex'));
      const choice = wasm.ResourceVoteChoice.TowardsIdentity(identityId);

      const obj = choice.toObject();
      expect(obj).to.be.an('object');
      // Serde serializes enum variants as { variantName: value } with camelCase
      expect(obj.towardsIdentity).to.not.be.undefined();
    });

    it('should serialize Abstain to object string', () => {
      const choice = wasm.ResourceVoteChoice.Abstain();

      const obj = choice.toObject();
      // Simple enum variants serialize to strings in serde
      expect(obj).to.equal('abstain');
    });

    it('should serialize Lock to object string', () => {
      const choice = wasm.ResourceVoteChoice.Lock();

      const obj = choice.toObject();
      // Simple enum variants serialize to strings in serde
      expect(obj).to.equal('lock');
    });
  });

  describe('fromObject()', () => {
    it('should deserialize TowardsIdentity from object', () => {
      const identityId = wasm.Identifier.fromBytes(Buffer.from(identityIdHex, 'hex'));
      const choice = wasm.ResourceVoteChoice.TowardsIdentity(identityId);

      const obj = choice.toObject();
      const restored = wasm.ResourceVoteChoice.fromObject(obj);
      expect(restored.voteType).to.equal(choice.voteType);
      expect(restored.value.toBase58()).to.equal(identityId.toBase58());
    });

    it('should deserialize Abstain from object', () => {
      const choice = wasm.ResourceVoteChoice.Abstain();

      const obj = choice.toObject();
      const restored = wasm.ResourceVoteChoice.fromObject(obj);
      expect(restored.voteType).to.equal('Abstain');
    });

    it('should deserialize Lock from object', () => {
      const choice = wasm.ResourceVoteChoice.Lock();

      const obj = choice.toObject();
      const restored = wasm.ResourceVoteChoice.fromObject(obj);
      expect(restored.voteType).to.equal('Lock');
    });
  });
});
