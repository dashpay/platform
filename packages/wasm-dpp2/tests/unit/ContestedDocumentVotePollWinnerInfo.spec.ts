import { Buffer } from 'buffer';
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
      const identityId = wasm.Identifier.fromHex(identityIdHex);
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

      const identityId = wasm.Identifier.fromHex(identityIdHex);
      const identity = new wasm.ContestedDocumentVotePollWinnerInfo('Identity', identityId);
      expect(identity.isWonByIdentity).to.be.true();
    });
  });

  describe('toJSON()', () => {
    it('should serialize NoWinner to JSON matching fixture', () => {
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('NoWinner');

      const json = info.toJSON();
      expect(json).to.deep.equal({ type: 'noWinner' });
    });

    it('should serialize WonByIdentity to JSON matching fixture', () => {
      const identityId = wasm.Identifier.fromHex(identityIdHex);
      const identityIdBase58 = identityId.toBase58();
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('WonByIdentity', identityId);

      const json = info.toJSON();
      expect(json).to.deep.equal({ type: 'wonByIdentity', data: identityIdBase58 });
    });

    it('should serialize Locked to JSON matching fixture', () => {
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('Locked');

      const json = info.toJSON();
      expect(json).to.deep.equal({ type: 'locked' });
    });
  });

  describe('fromJSON()', () => {
    it('should create NoWinner from JSON fixture and verify getters', () => {
      const restored = wasm.ContestedDocumentVotePollWinnerInfo.fromJSON({ type: 'noWinner' });

      expect(restored.kind).to.equal('NoWinner');
      expect(restored.isNoWinner).to.be.true();
      expect(restored.isWonByIdentity).to.be.false();
      expect(restored.isLocked).to.be.false();
      expect(restored.identityId).to.be.undefined();
    });

    it('should create WonByIdentity from JSON fixture and verify getters', () => {
      const identityId = wasm.Identifier.fromHex(identityIdHex);
      const identityIdBase58 = identityId.toBase58();

      const fixture = { type: 'wonByIdentity', data: identityIdBase58 };

      const restored = wasm.ContestedDocumentVotePollWinnerInfo.fromJSON(fixture);
      expect(restored.kind).to.equal('WonByIdentity');
      expect(restored.isWonByIdentity).to.be.true();
      expect(restored.isNoWinner).to.be.false();
      expect(restored.isLocked).to.be.false();
      expect(restored.identityId).to.not.be.undefined();
      expect(restored.identityId.toBase58()).to.equal(identityIdBase58);
    });

    it('should create Locked from JSON fixture and verify getters', () => {
      const restored = wasm.ContestedDocumentVotePollWinnerInfo.fromJSON({ type: 'locked' });

      expect(restored.kind).to.equal('Locked');
      expect(restored.isLocked).to.be.true();
      expect(restored.isNoWinner).to.be.false();
      expect(restored.isWonByIdentity).to.be.false();
      expect(restored.identityId).to.be.undefined();
    });
  });

  describe('toObject()', () => {
    it('should serialize NoWinner to Object matching fixture', () => {
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('NoWinner');

      const obj = info.toObject();
      expect(obj).to.deep.equal({ type: 'noWinner' });
    });

    it('should serialize WonByIdentity to Object with Uint8Array', () => {
      const identityId = wasm.Identifier.fromHex(identityIdHex);
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('WonByIdentity', identityId);

      const obj = info.toObject();
      expect(obj).to.be.an('object');
      expect(obj.type).to.equal('wonByIdentity');
      expect(obj.data).to.be.instanceOf(Uint8Array);
      expect(Buffer.from(obj.data).toString('hex')).to.equal(identityIdHex);
    });

    it('should serialize Locked to Object matching fixture', () => {
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('Locked');

      const obj = info.toObject();
      expect(obj).to.deep.equal({ type: 'locked' });
    });
  });

  describe('fromObject()', () => {
    it('should create NoWinner from Object fixture and verify getters', () => {
      const restored = wasm.ContestedDocumentVotePollWinnerInfo.fromObject({ type: 'noWinner' });

      expect(restored.kind).to.equal('NoWinner');
      expect(restored.isNoWinner).to.be.true();
      expect(restored.identityId).to.be.undefined();
    });

    it('should create WonByIdentity from Object fixture and verify getters', () => {
      const identityIdBytes = new Uint8Array(Buffer.from(identityIdHex, 'hex'));

      const fixture = { type: 'wonByIdentity', data: identityIdBytes };

      const restored = wasm.ContestedDocumentVotePollWinnerInfo.fromObject(fixture);
      expect(restored.kind).to.equal('WonByIdentity');
      expect(restored.isWonByIdentity).to.be.true();
      expect(restored.identityId).to.not.be.undefined();
      expect(restored.identityId.toHex()).to.equal(identityIdHex);
    });

    it('should create Locked from Object fixture and verify getters', () => {
      const restored = wasm.ContestedDocumentVotePollWinnerInfo.fromObject({ type: 'locked' });

      expect(restored.kind).to.equal('Locked');
      expect(restored.isLocked).to.be.true();
      expect(restored.identityId).to.be.undefined();
    });
  });
});
