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
    it('should serialize with $type tag and flat ResourceVote fields', () => {
      // Vote is internally tagged with `$type` (`$`-prefix because the level
      // also carries the inner ResourceVote's `$formatVersion`). The single
      // ResourceVote variant flattens its V0 body at the same level — no
      // `data` wrapper. VotePoll inside is also flat-tagged.
      const poll = createPoll();
      const choice = wasm.ResourceVoteChoice.TowardsIdentity(testIdentityId);
      const vote = new wasm.Vote(poll, choice);

      const json = vote.toJSON();

      expect(json.$type).to.equal('resourceVote');
      expect(json.$formatVersion).to.equal('0');
      expect(json.votePoll.$type).to.equal('contestedDocumentResourceVotePoll');
      expect(json.votePoll.contractId).to.equal(testContractId);
      expect(json.resourceVoteChoice).to.exist();

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

      expect(obj.$type).to.equal('resourceVote');
      expect(obj.$formatVersion).to.equal('0');
      expect(obj.votePoll.$type).to.equal('contestedDocumentResourceVotePoll');
      expect(obj.votePoll.contractId).to.be.instanceOf(Uint8Array);

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
