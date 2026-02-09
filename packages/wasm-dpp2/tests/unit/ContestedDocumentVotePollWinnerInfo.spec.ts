import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('ContestedDocumentVotePollWinnerInfo', () => {
  const identityIdHex = '1111111111111111111111111111111111111111111111111111111111111111';

  describe('constructor()', () => {
    it('should create NoWinner info', () => {
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('NoWinner');

      expect(info.kind).to.equal('NoWinner');
      expect(info.identityId).to.be.undefined();
      expect(info.isNoWinner).to.be.true();
      expect(info.isWonByIdentity).to.be.false();
      expect(info.isLocked).to.be.false();
    });

    it('should create WonByIdentity info', () => {
      const identityId = wasm.Identifier.fromBytes(Buffer.from(identityIdHex, 'hex'));
      const identityIdBase58 = identityId.toBase58();
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('WonByIdentity', identityId);

      expect(info.kind).to.equal('WonByIdentity');
      expect(info.identityId).to.not.be.undefined();
      expect(info.identityId.toBase58()).to.equal(identityIdBase58);
      expect(info.isNoWinner).to.be.false();
      expect(info.isWonByIdentity).to.be.true();
      expect(info.isLocked).to.be.false();
    });

    it('should create Locked info', () => {
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('Locked');

      expect(info.kind).to.equal('Locked');
      expect(info.identityId).to.be.undefined();
      expect(info.isNoWinner).to.be.false();
      expect(info.isWonByIdentity).to.be.false();
      expect(info.isLocked).to.be.true();
    });

    it('should accept alternative kind names', () => {
      const noWinner = new wasm.ContestedDocumentVotePollWinnerInfo('noWinner');
      expect(noWinner.isNoWinner).to.be.true();

      const locked = new wasm.ContestedDocumentVotePollWinnerInfo('LOCKED');
      expect(locked.isLocked).to.be.true();

      const identityId = wasm.Identifier.fromBytes(Buffer.from(identityIdHex, 'hex'));
      const identity = new wasm.ContestedDocumentVotePollWinnerInfo('Identity', identityId);
      expect(identity.isWonByIdentity).to.be.true();
    });
  });

  describe('toJSON()', () => {
    it('should round-trip NoWinner via toJSON/fromJSON', () => {
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('NoWinner');

      const json = info.toJSON();
      // Simple enum variants serialize to strings in serde
      expect(json).to.equal('NoWinner');

      const restored = wasm.ContestedDocumentVotePollWinnerInfo.fromJSON(json);
      expect(restored.kind).to.equal(info.kind);
      expect(restored.isNoWinner).to.be.true();
    });

    it('should round-trip WonByIdentity via toJSON/fromJSON', () => {
      const identityId = wasm.Identifier.fromBytes(Buffer.from(identityIdHex, 'hex'));
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('WonByIdentity', identityId);

      const json = info.toJSON();
      expect(json).to.be.an('object');
      // Serde serializes tuple variants as { VariantName: value }
      expect(json.WonByIdentity).to.be.a('string');

      const restored = wasm.ContestedDocumentVotePollWinnerInfo.fromJSON(json);
      expect(restored.kind).to.equal(info.kind);
      expect(restored.identityId.toBase58()).to.equal(info.identityId.toBase58());
    });

    it('should round-trip Locked via toJSON/fromJSON', () => {
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('Locked');

      const json = info.toJSON();
      // Simple enum variants serialize to strings in serde
      expect(json).to.equal('Locked');

      const restored = wasm.ContestedDocumentVotePollWinnerInfo.fromJSON(json);
      expect(restored.kind).to.equal(info.kind);
      expect(restored.isLocked).to.be.true();
    });
  });

  describe('toObject()', () => {
    it('should round-trip NoWinner via toObject/fromObject', () => {
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('NoWinner');

      const obj = info.toObject();
      // Simple enum variants serialize to strings in serde
      expect(obj).to.equal('NoWinner');

      const restored = wasm.ContestedDocumentVotePollWinnerInfo.fromObject(obj);
      expect(restored.kind).to.equal(info.kind);
    });

    it('should round-trip WonByIdentity via toObject/fromObject', () => {
      const identityId = wasm.Identifier.fromBytes(Buffer.from(identityIdHex, 'hex'));
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('WonByIdentity', identityId);

      const obj = info.toObject();
      expect(obj).to.be.an('object');
      // Serde serializes tuple variants as { VariantName: value }
      expect(obj.WonByIdentity).to.not.be.undefined();

      const restored = wasm.ContestedDocumentVotePollWinnerInfo.fromObject(obj);
      expect(restored.kind).to.equal(info.kind);
      expect(restored.identityId.toBase58()).to.equal(info.identityId.toBase58());
    });

    it('should round-trip Locked via toObject/fromObject', () => {
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('Locked');

      const obj = info.toObject();
      // Simple enum variants serialize to strings in serde
      expect(obj).to.equal('Locked');

      const restored = wasm.ContestedDocumentVotePollWinnerInfo.fromObject(obj);
      expect(restored.kind).to.equal(info.kind);
    });
  });
});
