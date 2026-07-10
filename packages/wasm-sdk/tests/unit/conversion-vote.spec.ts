import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

describe('Vote Conversions', () => {
  const testContractId = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';
  const testIdentityId = '2QjL594djCH2NyDsn45vd6yQjEDHupMKo7CEGVTHtQxU';

  before(async () => {
    await init();
  });

  describe('ResourceVoteChoice', () => {
    describe('TowardsIdentity', () => {
      it('should serialize to JSON with type tag and Base58 identity', () => {
        const choice = sdk.ResourceVoteChoice.TowardsIdentity(testIdentityId);
        const json = choice.toJSON();

        // Custom serde: flat `{$type, identity}` shape (no `data` wrapper —
        // see ResourceVoteChoice Serialize impl in rs-dpp).
        expect(json).to.deep.equal({ $type: 'towardsIdentity', identity: testIdentityId });

        choice.free();
      });

      it('should serialize to Object with type tag and Uint8Array identity', () => {
        const choice = sdk.ResourceVoteChoice.TowardsIdentity(testIdentityId);
        const obj = choice.toObject();

        expect(obj.$type).to.equal('towardsIdentity');
        expect(obj.identity).to.be.instanceOf(Uint8Array);

        choice.free();
      });

      it('should round-trip through JSON', () => {
        const choice = sdk.ResourceVoteChoice.TowardsIdentity(testIdentityId);
        const json = choice.toJSON();
        const restored = sdk.ResourceVoteChoice.fromJSON(json);

        expect(restored.voteType).to.equal('TowardsIdentity');
        expect(restored.value.toBase58()).to.equal(testIdentityId);

        choice.free();
        restored.free();
      });

      it('should round-trip through Object', () => {
        const choice = sdk.ResourceVoteChoice.TowardsIdentity(testIdentityId);
        const obj = choice.toObject();
        const restored = sdk.ResourceVoteChoice.fromObject(obj);

        expect(restored.voteType).to.equal('TowardsIdentity');
        expect(restored.value.toBase58()).to.equal(testIdentityId);

        choice.free();
        restored.free();
      });
    });

    describe('Abstain', () => {
      it('should serialize to JSON as object with type', () => {
        const choice = sdk.ResourceVoteChoice.Abstain();
        const json = choice.toJSON();

        expect(json).to.deep.equal({ $type: 'abstain' });

        choice.free();
      });

      it('should have correct voteType getter', () => {
        const choice = sdk.ResourceVoteChoice.Abstain();

        expect(choice.voteType).to.equal('Abstain');
        expect(choice.value).to.be.undefined();

        choice.free();
      });
    });

    describe('Lock', () => {
      it('should serialize to JSON as object with type', () => {
        const choice = sdk.ResourceVoteChoice.Lock();
        const json = choice.toJSON();

        expect(json).to.deep.equal({ $type: 'lock' });

        choice.free();
      });

      it('should have correct voteType getter', () => {
        const choice = sdk.ResourceVoteChoice.Lock();

        expect(choice.voteType).to.equal('Lock');
        expect(choice.value).to.be.undefined();

        choice.free();
      });
    });
  });

  describe('VotePoll', () => {
    const votePollOptions = {
      contractId: testContractId,
      documentTypeName: 'domain',
      indexName: 'parentNameAndLabel',
      indexValues: ['dash', 'alice'],
    };

    describe('toJSON()', () => {
      it('should serialize with internal type tag and flattened fields', () => {
        const poll = new sdk.VotePoll(votePollOptions);
        const json = poll.toJSON();

        // Internal serde tagging: `{$type, ...fields}` with no `data` wrapper.
        expect(json.$type).to.equal('contestedDocumentResourceVotePoll');
        expect(json.contractId).to.equal(testContractId);
        expect(json.documentTypeName).to.equal('domain');
        expect(json.indexName).to.equal('parentNameAndLabel');
        expect(json.indexValues).to.deep.equal(['dash', 'alice']);

        poll.free();
      });
    });

    describe('fromJSON()', () => {
      it('should deserialize from JSON fixture', () => {
        const fixture = {
          $type: 'contestedDocumentResourceVotePoll',
          contractId: testContractId,
          documentTypeName: 'domain',
          indexName: 'parentNameAndLabel',
          indexValues: ['dash', 'alice'],
        };

        const poll = sdk.VotePoll.fromJSON(fixture);

        expect(poll.contractId.toBase58()).to.equal(testContractId);
        expect(poll.documentTypeName).to.equal('domain');
        expect(poll.indexName).to.equal('parentNameAndLabel');

        poll.free();
      });

      it('should round-trip through JSON', () => {
        const poll = new sdk.VotePoll(votePollOptions);

        const json = poll.toJSON();
        const restored = sdk.VotePoll.fromJSON(json);
        const json2 = restored.toJSON();

        expect(json2).to.deep.equal(json);

        poll.free();
        restored.free();
      });
    });

    describe('toObject()', () => {
      it('should serialize with internal type tag and Uint8Array contractId', () => {
        const poll = new sdk.VotePoll(votePollOptions);
        const obj = poll.toObject();

        expect(obj.$type).to.equal('contestedDocumentResourceVotePoll');
        expect(obj.contractId).to.be.instanceOf(Uint8Array);
        expect(obj.documentTypeName).to.equal('domain');
        expect(obj.indexName).to.equal('parentNameAndLabel');

        poll.free();
      });
    });

    describe('fromObject()', () => {
      it('should round-trip through Object', () => {
        const poll = new sdk.VotePoll(votePollOptions);

        const obj = poll.toObject();
        const restored = sdk.VotePoll.fromObject(obj);

        expect(restored.contractId.toBase58()).to.equal(testContractId);
        expect(restored.documentTypeName).to.equal('domain');
        expect(restored.indexName).to.equal('parentNameAndLabel');

        poll.free();
        restored.free();
      });
    });
  });

  describe('ResourceVote', () => {
    it('should serialize to JSON with nested poll and choice', () => {
      const poll = new sdk.VotePoll({
        contractId: testContractId,
        documentTypeName: 'domain',
        indexName: 'parentNameAndLabel',
        indexValues: ['dash', 'alice'],
      });
      const choice = sdk.ResourceVoteChoice.TowardsIdentity(testIdentityId);
      const vote = new sdk.ResourceVote(poll, choice);

      const json = vote.toJSON();

      expect(json.$formatVersion).to.equal('0');
      expect(json.votePoll).to.exist();
      expect(json.votePoll.$type).to.equal('contestedDocumentResourceVotePoll');
      expect(json.votePoll.contractId).to.equal(testContractId);
      expect(json.resourceVoteChoice).to.exist();

      vote.free();
    });

    it('should round-trip through JSON', () => {
      const poll = new sdk.VotePoll({
        contractId: testContractId,
        documentTypeName: 'domain',
        indexName: 'parentNameAndLabel',
        indexValues: ['dash', 'bob'],
      });
      const choice = sdk.ResourceVoteChoice.Abstain();
      const vote = new sdk.ResourceVote(poll, choice);

      const json = vote.toJSON();
      const restored = sdk.ResourceVote.fromJSON(json);
      const json2 = restored.toJSON();

      expect(json2).to.deep.equal(json);

      vote.free();
      restored.free();
    });

    it('should round-trip through Object', () => {
      const poll = new sdk.VotePoll({
        contractId: testContractId,
        documentTypeName: 'domain',
        indexName: 'parentNameAndLabel',
        indexValues: ['dash', 'charlie'],
      });
      const choice = sdk.ResourceVoteChoice.Lock();
      const vote = new sdk.ResourceVote(poll, choice);

      const obj = vote.toObject();
      const restored = sdk.ResourceVote.fromObject(obj);

      expect(restored.choice.voteType).to.equal('Lock');
      expect(restored.poll.contractId.toBase58()).to.equal(testContractId);

      vote.free();
      restored.free();
    });
  });

  describe('Vote', () => {
    it('should serialize to JSON with resourceVote $type tag', () => {
      const poll = new sdk.VotePoll({
        contractId: testContractId,
        documentTypeName: 'domain',
        indexName: 'parentNameAndLabel',
        indexValues: ['dash', 'alice'],
      });
      const choice = sdk.ResourceVoteChoice.TowardsIdentity(testIdentityId);
      const vote = new sdk.Vote(poll, choice);

      const json = vote.toJSON();

      // Vote uses internal tagging on `$type` (system field convention).
      expect(json.$type).to.equal('resourceVote');
      expect(json.$formatVersion).to.equal('0');
      expect(json.votePoll.$type).to.equal('contestedDocumentResourceVotePoll');
      expect(json.votePoll.contractId).to.equal(testContractId);
      expect(json.resourceVoteChoice).to.exist();

      vote.free();
    });

    it('should round-trip through JSON', () => {
      const poll = new sdk.VotePoll({
        contractId: testContractId,
        documentTypeName: 'domain',
        indexName: 'parentNameAndLabel',
        indexValues: ['dash', 'dave'],
      });
      const choice = sdk.ResourceVoteChoice.Abstain();
      const vote = new sdk.Vote(poll, choice);

      const json = vote.toJSON();
      const restored = sdk.Vote.fromJSON(json);
      const json2 = restored.toJSON();

      expect(json2).to.deep.equal(json);

      vote.free();
      restored.free();
    });

    it('should round-trip through Object', () => {
      const poll = new sdk.VotePoll({
        contractId: testContractId,
        documentTypeName: 'domain',
        indexName: 'parentNameAndLabel',
        indexValues: ['dash', 'eve'],
      });
      const choice = sdk.ResourceVoteChoice.Lock();
      const vote = new sdk.Vote(poll, choice);

      const obj = vote.toObject();
      const restored = sdk.Vote.fromObject(obj);

      expect(restored.choice.voteType).to.equal('Lock');
      expect(restored.poll.documentTypeName).to.equal('domain');

      vote.free();
      restored.free();
    });

    describe('getters', () => {
      it('should expose choice and poll getters', () => {
        const poll = new sdk.VotePoll({
          contractId: testContractId,
          documentTypeName: 'domain',
          indexName: 'parentNameAndLabel',
          indexValues: ['dash', 'alice'],
        });
        const choice = sdk.ResourceVoteChoice.TowardsIdentity(testIdentityId);
        const vote = new sdk.Vote(poll, choice);

        expect(vote.choice.voteType).to.equal('TowardsIdentity');
        expect(vote.choice.value.toBase58()).to.equal(testIdentityId);
        expect(vote.poll.contractId.toBase58()).to.equal(testContractId);
        expect(vote.poll.documentTypeName).to.equal('domain');

        vote.free();
      });
    });
  });
});
