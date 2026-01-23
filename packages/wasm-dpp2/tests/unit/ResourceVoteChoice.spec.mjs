import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('ResourceVoteChoice', () => {
  const identityIdHex = '1111111111111111111111111111111111111111111111111111111111111111';

  describe('static constructors', () => {
    it('should create TowardsIdentity choice', () => {
      const identityId = wasm.Identifier.fromBytes(Buffer.from(identityIdHex, 'hex'));
      const choice = wasm.ResourceVoteChoice.TowardsIdentity(identityId);

      expect(choice.voteType).to.equal('TowardsIdentity');
      expect(choice.value).to.not.be.undefined();
    });

    it('should create Abstain choice', () => {
      const choice = wasm.ResourceVoteChoice.Abstain();

      expect(choice.voteType).to.equal('Abstain');
      expect(choice.value).to.be.undefined();
    });

    it('should create Lock choice', () => {
      const choice = wasm.ResourceVoteChoice.Lock();

      expect(choice.voteType).to.equal('Lock');
      expect(choice.value).to.be.undefined();
    });
  });

  describe('type properties', () => {
    it('should return correct __type', () => {
      const choice = wasm.ResourceVoteChoice.Abstain();

      expect(choice.__type).to.equal('ResourceVoteChoice');
    });
  });

  describe('conversion methods', () => {
    it('should round-trip TowardsIdentity via toJSON/fromJSON', () => {
      const identityId = wasm.Identifier.fromBytes(Buffer.from(identityIdHex, 'hex'));
      const choice = wasm.ResourceVoteChoice.TowardsIdentity(identityId);

      const json = choice.toJSON();
      expect(json).to.be.an('object');

      const restored = wasm.ResourceVoteChoice.fromJSON(json);
      expect(restored.voteType).to.equal(choice.voteType);
    });

    it('should round-trip Abstain via toJSON/fromJSON', () => {
      const choice = wasm.ResourceVoteChoice.Abstain();

      const json = choice.toJSON();
      // Simple enum variants serialize to strings in serde
      expect(json).to.equal('abstain');

      const restored = wasm.ResourceVoteChoice.fromJSON(json);
      expect(restored.voteType).to.equal('Abstain');
    });

    it('should round-trip Lock via toJSON/fromJSON', () => {
      const choice = wasm.ResourceVoteChoice.Lock();

      const json = choice.toJSON();
      // Simple enum variants serialize to strings in serde
      expect(json).to.equal('lock');

      const restored = wasm.ResourceVoteChoice.fromJSON(json);
      expect(restored.voteType).to.equal('Lock');
    });

    it('should round-trip TowardsIdentity via toObject/fromObject', () => {
      const identityId = wasm.Identifier.fromBytes(Buffer.from(identityIdHex, 'hex'));
      const choice = wasm.ResourceVoteChoice.TowardsIdentity(identityId);

      const obj = choice.toObject();
      expect(obj).to.be.an('object');
      // Serde serializes enum variants as { variantName: value } with camelCase
      expect(obj.towardsIdentity).to.not.be.undefined();

      const restored = wasm.ResourceVoteChoice.fromObject(obj);
      expect(restored.voteType).to.equal(choice.voteType);
      expect(restored.value.toBase58()).to.equal(identityId.toBase58());
    });

    it('should round-trip Abstain via toObject/fromObject', () => {
      const choice = wasm.ResourceVoteChoice.Abstain();

      const obj = choice.toObject();
      // Simple enum variants serialize to strings in serde
      expect(obj).to.equal('abstain');

      const restored = wasm.ResourceVoteChoice.fromObject(obj);
      expect(restored.voteType).to.equal('Abstain');
    });

    it('should round-trip Lock via toObject/fromObject', () => {
      const choice = wasm.ResourceVoteChoice.Lock();

      const obj = choice.toObject();
      // Simple enum variants serialize to strings in serde
      expect(obj).to.equal('lock');

      const restored = wasm.ResourceVoteChoice.fromObject(obj);
      expect(restored.voteType).to.equal('Lock');
    });
  });
});
