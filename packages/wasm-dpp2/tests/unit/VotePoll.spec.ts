import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('VotePoll', () => {
  const testContractId = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

  const votePollOptions = {
    contractId: testContractId,
    documentTypeName: 'domain',
    indexName: 'parentNameAndLabel',
    indexValues: ['dash', 'alice'],
  };

  describe('toJSON()', () => {
    it('should serialize with type tag and flat fields', () => {
      // VotePoll is internally tagged (`tag = "$type"`) — no `data` wrapper.
      // Plain `type` because the level has no other `$`-prefixed fields.
      const poll = new wasm.VotePoll(votePollOptions);
      const json = poll.toJSON();

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

      const poll = wasm.VotePoll.fromJSON(fixture);

      expect(poll.contractId.toBase58()).to.equal(testContractId);
      expect(poll.documentTypeName).to.equal('domain');
      expect(poll.indexName).to.equal('parentNameAndLabel');

      poll.free();
    });

    it('should round-trip through JSON', () => {
      const poll = new wasm.VotePoll(votePollOptions);

      const json = poll.toJSON();
      const restored = wasm.VotePoll.fromJSON(json);
      const json2 = restored.toJSON();

      expect(json2).to.deep.equal(json);

      poll.free();
      restored.free();
    });
  });

  describe('toObject()', () => {
    it('should serialize with type tag and Uint8Array contractId at top level', () => {
      const poll = new wasm.VotePoll(votePollOptions);
      const obj = poll.toObject();

      expect(obj.$type).to.equal('contestedDocumentResourceVotePoll');
      expect(obj.contractId).to.be.instanceOf(Uint8Array);
      expect(obj.documentTypeName).to.equal('domain');
      expect(obj.indexName).to.equal('parentNameAndLabel');

      poll.free();
    });
  });

  describe('fromObject()', () => {
    it('should deserialize from Object fixture', () => {
      const poll = new wasm.VotePoll(votePollOptions);
      const obj = poll.toObject();

      const restored = wasm.VotePoll.fromObject(obj);

      expect(restored.contractId.toBase58()).to.equal(testContractId);
      expect(restored.documentTypeName).to.equal('domain');
      expect(restored.indexName).to.equal('parentNameAndLabel');

      poll.free();
      restored.free();
    });
  });

  describe('getters', () => {
    it('should expose contractId as Identifier', () => {
      const poll = new wasm.VotePoll(votePollOptions);

      expect(poll.contractId.__type).to.equal('Identifier');
      expect(poll.contractId.toBase58()).to.equal(testContractId);

      poll.free();
    });

    it('should expose documentTypeName', () => {
      const poll = new wasm.VotePoll(votePollOptions);

      expect(poll.documentTypeName).to.equal('domain');

      poll.free();
    });

    it('should expose indexName', () => {
      const poll = new wasm.VotePoll(votePollOptions);

      expect(poll.indexName).to.equal('parentNameAndLabel');

      poll.free();
    });
  });

  describe('yes/no vote poll', () => {
    // A yes/no poll has no constructor of its own; it comes from its object or JSON form.
    const yesNoPollJSON = {
      $type: 'yesNoVotePoll',
      resourcePath: ['wcHBwQ=='],
      supermajorityNumerator: 2,
      supermajorityDenominator: 3,
      minimumVotingPower: { $type: 'absolute', votingPower: 400 },
    };

    it('should serialize toJSON() with the yesNoVotePoll tag and its rule', () => {
      const poll = wasm.VotePoll.fromJSON(yesNoPollJSON);
      const json = poll.toJSON();

      expect(json.$type).to.equal('yesNoVotePoll');
      expect(json.resourcePath).to.deep.equal(['wcHBwQ==']);
      expect(json.supermajorityNumerator).to.equal(2);
      expect(json.supermajorityDenominator).to.equal(3);
      expect(json.minimumVotingPower).to.deep.equal({ $type: 'absolute', votingPower: 400 });

      poll.free();
    });

    it('should round-trip a share minimum through fromJSON() and toJSON()', () => {
      const fixture = {
        ...yesNoPollJSON,
        minimumVotingPower: {
          $type: 'fractionOfTotal',
          numerator: 1,
          denominator: 3,
          rounding: 'up',
        },
      };
      const poll = wasm.VotePoll.fromJSON(fixture);

      expect(poll.toJSON()).to.deep.equal(fixture);

      poll.free();
    });

    it('should serialize toObject() with Uint8Array path segments and round-trip fromObject()', () => {
      const poll = wasm.VotePoll.fromJSON(yesNoPollJSON);
      const obj = poll.toObject();

      expect(obj.$type).to.equal('yesNoVotePoll');
      expect(obj.resourcePath).to.have.lengthOf(1);
      expect(obj.resourcePath[0]).to.be.instanceOf(Uint8Array);
      expect(Array.from(obj.resourcePath[0])).to.deep.equal([0xc1, 0xc1, 0xc1, 0xc1]);
      expect(obj.minimumVotingPower.$type).to.equal('absolute');

      const restored = wasm.VotePoll.fromObject(obj);
      expect(restored.toObject()).to.deep.equal(obj);

      poll.free();
      restored.free();
    });

    it('should report its kind and refuse the contested poll getters', () => {
      const poll = wasm.VotePoll.fromJSON(yesNoPollJSON);

      expect(poll.kind).to.equal('yesNoVotePoll');
      expect(() => poll.contractId).to.throw();
      expect(() => poll.documentTypeName).to.throw();
      expect(() => poll.indexName).to.throw();
      expect(() => poll.indexValues).to.throw();

      poll.free();
    });
  });
});
