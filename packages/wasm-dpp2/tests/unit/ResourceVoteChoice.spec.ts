import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('ResourceVoteChoice', () => {
  const identityIdHex = '1111111111111111111111111111111111111111111111111111111111111111';

  describe('TowardsIdentity()', () => {
    it('should create TowardsIdentity choice', () => {
      const identityId = wasm.Identifier.fromHex(identityIdHex);
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

  describe('toJSON()', () => {
    it('should serialize TowardsIdentity to JSON matching fixture', () => {
      const identityId = wasm.Identifier.fromHex(identityIdHex);
      const identityIdBase58 = identityId.toBase58();
      const choice = wasm.ResourceVoteChoice.TowardsIdentity(identityId);

      const json = choice.toJSON();
      expect(json).to.deep.equal({ type: 'towardsIdentity', data: identityIdBase58 });
    });

    it('should serialize Abstain to JSON', () => {
      const choice = wasm.ResourceVoteChoice.Abstain();

      const json = choice.toJSON();
      expect(json).to.deep.equal({ type: 'abstain' });
    });

    it('should serialize Lock to JSON', () => {
      const choice = wasm.ResourceVoteChoice.Lock();

      const json = choice.toJSON();
      expect(json).to.deep.equal({ type: 'lock' });
    });
  });

  describe('fromJSON()', () => {
    it('should create TowardsIdentity from JSON fixture and verify getters', () => {
      const identityId = wasm.Identifier.fromHex(identityIdHex);
      const identityIdBase58 = identityId.toBase58();

      const fixture = { type: 'towardsIdentity', data: identityIdBase58 };

      const restored = wasm.ResourceVoteChoice.fromJSON(fixture);
      expect(restored.voteType).to.equal('TowardsIdentity');
      expect(restored.value).to.not.be.undefined();
      expect(restored.value.toBase58()).to.equal(identityIdBase58);
    });

    it('should create Abstain from JSON fixture', () => {
      const restored = wasm.ResourceVoteChoice.fromJSON({ type: 'abstain' });
      expect(restored.voteType).to.equal('Abstain');
      expect(restored.value).to.be.undefined();
    });

    it('should create Lock from JSON fixture', () => {
      const restored = wasm.ResourceVoteChoice.fromJSON({ type: 'lock' });
      expect(restored.voteType).to.equal('Lock');
      expect(restored.value).to.be.undefined();
    });
  });

  describe('toObject()', () => {
    it('should serialize TowardsIdentity to object with Uint8Array', () => {
      const identityId = wasm.Identifier.fromHex(identityIdHex);
      const choice = wasm.ResourceVoteChoice.TowardsIdentity(identityId);

      const obj = choice.toObject();
      expect(obj).to.be.an('object');
      expect(obj.type).to.equal('towardsIdentity');
      expect(obj.data).to.be.instanceOf(Uint8Array);
      expect(Buffer.from(obj.data).toString('hex')).to.equal(identityIdHex);
    });

    it('should serialize Abstain to object', () => {
      const choice = wasm.ResourceVoteChoice.Abstain();

      const obj = choice.toObject();
      expect(obj).to.deep.equal({ type: 'abstain' });
    });

    it('should serialize Lock to object', () => {
      const choice = wasm.ResourceVoteChoice.Lock();

      const obj = choice.toObject();
      expect(obj).to.deep.equal({ type: 'lock' });
    });
  });

  describe('fromObject()', () => {
    it('should create TowardsIdentity from object fixture and verify getters', () => {
      const identityIdBytes = new Uint8Array(Buffer.from(identityIdHex, 'hex'));

      const fixture = { type: 'towardsIdentity', data: identityIdBytes };

      const restored = wasm.ResourceVoteChoice.fromObject(fixture);
      expect(restored.voteType).to.equal('TowardsIdentity');
      expect(restored.value).to.not.be.undefined();
      expect(restored.value.toHex()).to.equal(identityIdHex);
    });

    it('should create Abstain from object fixture', () => {
      const restored = wasm.ResourceVoteChoice.fromObject({ type: 'abstain' });
      expect(restored.voteType).to.equal('Abstain');
      expect(restored.value).to.be.undefined();
    });

    it('should create Lock from object fixture', () => {
      const restored = wasm.ResourceVoteChoice.fromObject({ type: 'lock' });
      expect(restored.voteType).to.equal('Lock');
      expect(restored.value).to.be.undefined();
    });
  });
});
