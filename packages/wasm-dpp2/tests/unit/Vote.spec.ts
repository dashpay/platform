import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('Vote', () => {
  const testContractId = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';
  const testIdentityId = '2QjL594djCH2NyDsn45vd6yQjEDHupMKo7CEGVTHtQxU';

  function createPoll(indexValues = ['dash', 'alice']) {
    return new wasm.VotePoll({
      contractId: testContractId,
      documentTypeName: 'domain',
      indexName: 'parentNameAndLabel',
      indexValues,
    });
  }

  describe('toJSON()', () => {
    it('should serialize with resourceVote type tag', () => {
      const poll = createPoll();
      const choice = wasm.ResourceVoteChoice.TowardsIdentity(testIdentityId);
      const vote = new wasm.Vote(poll, choice);

      const json = vote.toJSON();

      expect(json.type).to.equal('resourceVote');
      expect(json.data).to.exist();
      expect(json.data.$formatVersion).to.equal('0');
      expect(json.data.votePoll.type).to.equal('contestedDocumentResourceVotePoll');
      expect(json.data.votePoll.data.contractId).to.equal(testContractId);
      expect(json.data.resourceVoteChoice).to.exist();

      vote.free();
    });
  });

  describe('fromJSON()', () => {
    it('should round-trip through JSON', () => {
      const poll = createPoll(['dash', 'dave']);
      const choice = wasm.ResourceVoteChoice.Abstain();
      const vote = new wasm.Vote(poll, choice);

      const json = vote.toJSON();
      const restored = wasm.Vote.fromJSON(json);
      const json2 = restored.toJSON();

      expect(json2).to.deep.equal(json);

      vote.free();
      restored.free();
    });
  });

  describe('toObject()', () => {
    it('should serialize with resourceVote type tag containing Uint8Array', () => {
      const poll = createPoll(['dash', 'eve']);
      const choice = wasm.ResourceVoteChoice.Lock();
      const vote = new wasm.Vote(poll, choice);

      const obj = vote.toObject();

      expect(obj.type).to.equal('resourceVote');
      expect(obj.data).to.exist();
      expect(obj.data.$formatVersion).to.equal('0');
      expect(obj.data.votePoll.type).to.equal('contestedDocumentResourceVotePoll');
      expect(obj.data.votePoll.data.contractId).to.be.instanceOf(Uint8Array);

      vote.free();
    });
  });

  describe('getters', () => {
    it('should expose choice and poll getters', () => {
      const poll = createPoll();
      const choice = wasm.ResourceVoteChoice.TowardsIdentity(testIdentityId);
      const vote = new wasm.Vote(poll, choice);

      expect(vote.choice.voteType).to.equal('TowardsIdentity');
      expect(vote.poll.contractId.toBase58()).to.equal(testContractId);
      expect(vote.poll.documentTypeName).to.equal('domain');

      vote.free();
    });
  });
});
