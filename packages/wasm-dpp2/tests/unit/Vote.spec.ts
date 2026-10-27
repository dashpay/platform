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

  describe('yes/no vote', () => {
    // A yes/no vote carries its poll's fields untagged.
    const yesNoVoteJSON = {
      $type: 'yesNoVote',
      $formatVersion: '0',
      votePoll: {
        resourcePath: ['wcHBwQ=='],
        supermajorityNumerator: 2,
        supermajorityDenominator: 3,
        minimumVotingPower: { $type: 'absolute', votingPower: 400 },
      },
      voteChoice: 'yes',
    };

    it('should serialize toJSON() with the yesNoVote tag and the answer', () => {
      const vote = wasm.Vote.fromJSON(yesNoVoteJSON);
      const json = vote.toJSON();

      expect(json.$type).to.equal('yesNoVote');
      expect(json.$formatVersion).to.equal('0');
      expect(json.votePoll).to.deep.equal(yesNoVoteJSON.votePoll);
      expect(json.voteChoice).to.equal('yes');
      expect(json).to.deep.equal(yesNoVoteJSON);

      vote.free();
    });

    it('should round-trip through toObject() and fromObject()', () => {
      const vote = wasm.Vote.fromJSON(yesNoVoteJSON);
      const obj = vote.toObject();

      expect(obj.$type).to.equal('yesNoVote');
      expect(obj.votePoll.resourcePath[0]).to.be.instanceOf(Uint8Array);
      expect(obj.voteChoice).to.equal('yes');

      const restored = wasm.Vote.fromObject(obj);
      expect(restored.toObject()).to.deep.equal(obj);

      vote.free();
      restored.free();
    });

    it('should expose its answer and poll, and refuse the resource vote choice', () => {
      const vote = wasm.Vote.fromJSON(yesNoVoteJSON);

      expect(vote.yesNoChoice).to.equal('yes');
      expect(vote.poll.kind).to.equal('yesNoVotePoll');
      expect(() => vote.choice).to.throw();

      vote.free();
    });

    it('should refuse the yes/no answer of a resource vote', () => {
      const vote = new wasm.Vote(createPoll(), wasm.ResourceVoteChoice.Abstain());

      expect(() => vote.yesNoChoice).to.throw();

      vote.free();
    });

    it('should refuse a resource vote on a yes/no poll', () => {
      const yesNoPoll = wasm.VotePoll.fromJSON({
        $type: 'yesNoVotePoll',
        ...yesNoVoteJSON.votePoll,
      });

      expect(() => new wasm.Vote(yesNoPoll, wasm.ResourceVoteChoice.Abstain())).to.throw();

      const vote = new wasm.Vote(createPoll(), wasm.ResourceVoteChoice.Abstain());
      expect(() => {
        vote.poll = yesNoPoll;
      }).to.throw();

      vote.free();
      yesNoPoll.free();
    });
  });
});
