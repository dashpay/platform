import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('ResourceVote', () => {
  const testContractId = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';
  const testIdentityId = '2QjL594djCH2NyDsn45vd6yQjEDHupMKo7CEGVTHtQxU';

  function createPoll() {
    return new wasm.VotePoll({
      contractId: testContractId,
      documentTypeName: 'domain',
      indexName: 'parentNameAndLabel',
      indexValues: ['dash', 'alice'],
    });
  }

  describe('toJSON()', () => {
    it('should serialize with nested poll and choice', () => {
      const poll = createPoll();
      const choice = wasm.ResourceVoteChoice.TowardsIdentity(testIdentityId);
      const vote = new wasm.ResourceVote(poll, choice);

      const json = vote.toJSON();

      expect(json.$formatVersion).to.equal('0');
      expect(json.votePoll).to.exist();
      expect(json.votePoll.contestedDocumentResourceVotePoll.contractId).to.equal(testContractId);
      expect(json.resourceVoteChoice).to.exist();

      vote.free();
    });

    it('should serialize Abstain choice as string', () => {
      const poll = createPoll();
      const choice = wasm.ResourceVoteChoice.Abstain();
      const vote = new wasm.ResourceVote(poll, choice);

      const json = vote.toJSON();

      expect(json.resourceVoteChoice).to.equal('abstain');

      vote.free();
    });
  });

  describe('fromJSON()', () => {
    it('should round-trip through JSON', () => {
      const poll = createPoll();
      const choice = wasm.ResourceVoteChoice.Abstain();
      const vote = new wasm.ResourceVote(poll, choice);

      const json = vote.toJSON();
      const restored = wasm.ResourceVote.fromJSON(json);
      const json2 = restored.toJSON();

      expect(json2).to.deep.equal(json);

      vote.free();
      restored.free();
    });
  });

  describe('toObject()', () => {
    it('should serialize with Uint8Array contractId in poll', () => {
      const poll = createPoll();
      const choice = wasm.ResourceVoteChoice.Lock();
      const vote = new wasm.ResourceVote(poll, choice);

      const obj = vote.toObject();

      expect(obj.$formatVersion).to.equal('0');
      expect(obj.votePoll).to.exist();
      expect(obj.votePoll.contestedDocumentResourceVotePoll.contractId).to.be.instanceOf(Uint8Array);
      expect(obj.resourceVoteChoice).to.equal('lock');

      vote.free();
    });
  });

  describe('getters', () => {
    it('should expose choice getter', () => {
      const poll = createPoll();
      const choice = wasm.ResourceVoteChoice.TowardsIdentity(testIdentityId);
      const vote = new wasm.ResourceVote(poll, choice);

      expect(vote.choice.voteType).to.equal('TowardsIdentity');

      vote.free();
    });

    it('should expose poll getter', () => {
      const poll = createPoll();
      const choice = wasm.ResourceVoteChoice.Abstain();
      const vote = new wasm.ResourceVote(poll, choice);

      expect(vote.poll.contractId.toBase58()).to.equal(testContractId);
      expect(vote.poll.documentTypeName).to.equal('domain');

      vote.free();
    });
  });
});
