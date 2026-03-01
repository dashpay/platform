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
    it('should serialize with contestedDocumentResourceVotePoll wrapper', () => {
      const poll = new wasm.VotePoll(votePollOptions);
      const json = poll.toJSON();

      const inner = json.contestedDocumentResourceVotePoll;
      expect(inner).to.exist();
      expect(inner.contractId).to.equal(testContractId);
      expect(inner.documentTypeName).to.equal('domain');
      expect(inner.indexName).to.equal('parentNameAndLabel');
      expect(inner.indexValues).to.deep.equal(['dash', 'alice']);

      poll.free();
    });
  });

  describe('fromJSON()', () => {
    it('should deserialize from JSON fixture', () => {
      const fixture = {
        contestedDocumentResourceVotePoll: {
          contractId: testContractId,
          documentTypeName: 'domain',
          indexName: 'parentNameAndLabel',
          indexValues: ['dash', 'alice'],
        },
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
    it('should serialize with Uint8Array contractId in wrapper', () => {
      const poll = new wasm.VotePoll(votePollOptions);
      const obj = poll.toObject();

      const inner = obj.contestedDocumentResourceVotePoll;
      expect(inner).to.exist();
      expect(inner.contractId).to.be.instanceOf(Uint8Array);
      expect(inner.documentTypeName).to.equal('domain');
      expect(inner.indexName).to.equal('parentNameAndLabel');

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
});
