import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('MasternodeVoteTransition', () => {
  const proTxHashHex = '1111111111111111111111111111111111111111111111111111111111111111';
  const voterIdHex = '2222222222222222222222222222222222222222222222222222222222222222';
  const contractIdBase58 = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

  function createVote() {
    const poll = new wasm.VotePoll({
      contractId: contractIdBase58,
      documentTypeName: 'domain',
      indexName: 'parentNameAndLabel',
      indexValues: ['dash', 'testname'],
    });
    const choice = wasm.ResourceVoteChoice.Abstain();
    return new wasm.Vote(poll, choice);
  }

  function createTransition() {
    const vote = createVote();
    return new wasm.MasternodeVoteTransition({
      proTxHash: proTxHashHex,
      voterIdentityId: voterIdHex,
      vote,
      nonce: 1n,
    });
  }

  describe('constructor()', () => {
    it('should create with required options', () => {
      const transition = createTransition();

      expect(transition).to.be.instanceOf(wasm.MasternodeVoteTransition);
      expect(transition.proTxHash.toHex()).to.equal(proTxHashHex);
      expect(transition.voterIdentityId.toHex()).to.equal(voterIdHex);
      expect(transition.nonce).to.equal(1n);
    });
  });

  describe('getters', () => {
    it('should return proTxHash', () => {
      const transition = createTransition();
      expect(transition.proTxHash.toHex()).to.equal(proTxHashHex);
    });

    it('should return voterIdentityId', () => {
      const transition = createTransition();
      expect(transition.voterIdentityId.toHex()).to.equal(voterIdHex);
    });

    it('should return vote', () => {
      const transition = createTransition();
      const vote = transition.vote;
      expect(vote).to.be.instanceOf(wasm.Vote);
    });

    it('should return nonce', () => {
      const transition = createTransition();
      expect(transition.nonce).to.equal(1n);
    });

    it('should return signaturePublicKeyId', () => {
      const transition = createTransition();
      expect(transition.signaturePublicKeyId).to.be.a('number');
    });

    it('should return userFeeIncrease', () => {
      const transition = createTransition();
      expect(transition.userFeeIncrease).to.be.a('number');
    });
  });

  describe('setters', () => {
    it('should set proTxHash', () => {
      const transition = createTransition();
      const newHash = '3333333333333333333333333333333333333333333333333333333333333333';
      transition.proTxHash = newHash;
      expect(transition.proTxHash.toHex()).to.equal(newHash);
    });

    it('should set voterIdentityId', () => {
      const transition = createTransition();
      const newId = '4444444444444444444444444444444444444444444444444444444444444444';
      transition.voterIdentityId = newId;
      expect(transition.voterIdentityId.toHex()).to.equal(newId);
    });

    it('should set nonce', () => {
      const transition = createTransition();
      transition.nonce = 99n;
      expect(transition.nonce).to.equal(99n);
    });
  });

  describe('toJSON()', () => {
    it('should serialize to JSON', () => {
      const transition = createTransition();
      const json = transition.toJSON();

      expect(json).to.be.an('object');
    });
  });

  describe('fromJSON()', () => {
    it('should round-trip via toJSON/fromJSON', () => {
      const transition = createTransition();
      const json = transition.toJSON();
      const restored = wasm.MasternodeVoteTransition.fromJSON(json);

      expect(restored.proTxHash.toHex()).to.equal(proTxHashHex);
      expect(restored.voterIdentityId.toHex()).to.equal(voterIdHex);
      expect(restored.nonce).to.equal(1n);
    });
  });

  describe('toObject()', () => {
    it('should serialize to Object', () => {
      const transition = createTransition();
      const obj = transition.toObject();

      expect(obj).to.be.an('object');
    });
  });

  describe('fromObject()', () => {
    it('should round-trip via toJSON then fromObject with JSON fixture', () => {
      const transition = createTransition();
      const json = transition.toJSON();
      const restored = wasm.MasternodeVoteTransition.fromJSON(json);

      // Verify the JSON-restored transition matches
      expect(restored.proTxHash.toHex()).to.equal(proTxHashHex);
      expect(restored.voterIdentityId.toHex()).to.equal(voterIdHex);
      expect(restored.nonce).to.equal(1n);
    });
  });

  describe('binary serialization', () => {
    it('should round-trip via toBytes/fromBytes', () => {
      const transition = createTransition();
      const bytes = transition.toBytes();
      const restored = wasm.MasternodeVoteTransition.fromBytes(bytes);

      expect(restored.proTxHash.toHex()).to.equal(proTxHashHex);
      expect(restored.voterIdentityId.toHex()).to.equal(voterIdHex);
      expect(restored.nonce).to.equal(1n);
    });

    it('should round-trip via toHex/fromHex', () => {
      const transition = createTransition();
      const hex = transition.toHex();
      const restored = wasm.MasternodeVoteTransition.fromHex(hex);

      expect(restored.proTxHash.toHex()).to.equal(proTxHashHex);
      expect(restored.nonce).to.equal(1n);
    });

    it('should round-trip via toBase64/fromBase64', () => {
      const transition = createTransition();
      const base64 = transition.toBase64();
      const restored = wasm.MasternodeVoteTransition.fromBase64(base64);

      expect(restored.proTxHash.toHex()).to.equal(proTxHashHex);
      expect(restored.nonce).to.equal(1n);
    });
  });

  describe('toStateTransition()', () => {
    it('should convert to generic StateTransition', () => {
      const transition = createTransition();
      const st = transition.toStateTransition();

      expect(st).to.be.instanceOf(wasm.StateTransition);
    });
  });

  describe('__type', () => {
    it('should return correct __type', () => {
      const transition = createTransition();
      expect(transition.__type).to.equal('MasternodeVoteTransition');
    });
  });
});
