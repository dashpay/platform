import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('ContestedDocumentVotePollWinnerInfo', () => {
  const identityIdHex = '1111111111111111111111111111111111111111111111111111111111111111';

  describe('constructor', () => {
    it('should create NoWinner info', () => {
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('none');

      expect(info.kind).to.equal('none');
      expect(info.identityId).to.be.null;
      expect(info.isNoWinner()).to.be.true;
      expect(info.isWonByIdentity()).to.be.false;
      expect(info.isLocked()).to.be.false;
    });

    it('should create WonByIdentity info', () => {
      const identityId = wasm.Identifier.fromBytes(Buffer.from(identityIdHex, 'hex'));
      const identityIdBase58 = identityId.toBase58();
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('identity', identityId);

      expect(info.kind).to.equal('identity');
      expect(info.identityId).to.not.be.null;
      expect(info.identityId.toBase58()).to.equal(identityIdBase58);
      expect(info.isNoWinner()).to.be.false;
      expect(info.isWonByIdentity()).to.be.true;
      expect(info.isLocked()).to.be.false;
    });

    it('should create Locked info', () => {
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('locked');

      expect(info.kind).to.equal('locked');
      expect(info.identityId).to.be.null;
      expect(info.isNoWinner()).to.be.false;
      expect(info.isWonByIdentity()).to.be.false;
      expect(info.isLocked()).to.be.true;
    });

    it('should accept alternative kind names', () => {
      const noWinner = new wasm.ContestedDocumentVotePollWinnerInfo('NoWinner');
      expect(noWinner.isNoWinner()).to.be.true;

      const locked = new wasm.ContestedDocumentVotePollWinnerInfo('LOCKED');
      expect(locked.isLocked()).to.be.true;

      const identityId = wasm.Identifier.fromBytes(Buffer.from(identityIdHex, 'hex'));
      const identity = new wasm.ContestedDocumentVotePollWinnerInfo('Identity', identityId);
      expect(identity.isWonByIdentity()).to.be.true;
    });
  });

  describe('conversion methods', () => {
    it('should round-trip NoWinner via toJSON/fromJSON', () => {
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('none');

      const json = info.toJSON();
      expect(json).to.be.an('object');
      expect(json.kind).to.equal('none');

      const restored = wasm.ContestedDocumentVotePollWinnerInfo.fromJSON(json);
      expect(restored.kind).to.equal(info.kind);
      expect(restored.isNoWinner()).to.be.true;
    });

    it('should round-trip WonByIdentity via toJSON/fromJSON', () => {
      const identityId = wasm.Identifier.fromBytes(Buffer.from(identityIdHex, 'hex'));
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('identity', identityId);

      const json = info.toJSON();
      expect(json).to.be.an('object');
      expect(json.kind).to.equal('identity');
      expect(json.identityId).to.be.a('string');

      const restored = wasm.ContestedDocumentVotePollWinnerInfo.fromJSON(json);
      expect(restored.kind).to.equal(info.kind);
      expect(restored.identityId.toBase58()).to.equal(info.identityId.toBase58());
    });

    it('should round-trip Locked via toJSON/fromJSON', () => {
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('locked');

      const json = info.toJSON();
      expect(json).to.be.an('object');
      expect(json.kind).to.equal('locked');

      const restored = wasm.ContestedDocumentVotePollWinnerInfo.fromJSON(json);
      expect(restored.kind).to.equal(info.kind);
      expect(restored.isLocked()).to.be.true;
    });

    it('should round-trip NoWinner via toObject/fromObject', () => {
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('none');

      const obj = info.toObject();
      expect(obj).to.be.an('object');

      const restored = wasm.ContestedDocumentVotePollWinnerInfo.fromObject(obj);
      expect(restored.kind).to.equal(info.kind);
    });

    it('should round-trip WonByIdentity via toObject/fromObject', () => {
      const identityId = wasm.Identifier.fromBytes(Buffer.from(identityIdHex, 'hex'));
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('identity', identityId);

      const obj = info.toObject();
      expect(obj).to.be.an('object');
      expect(obj.identityId).to.not.be.undefined;

      const restored = wasm.ContestedDocumentVotePollWinnerInfo.fromObject(obj);
      expect(restored.kind).to.equal(info.kind);
      expect(restored.identityId.toBase58()).to.equal(info.identityId.toBase58());
    });

    it('should round-trip Locked via toObject/fromObject', () => {
      const info = new wasm.ContestedDocumentVotePollWinnerInfo('locked');

      const obj = info.toObject();
      expect(obj).to.be.an('object');

      const restored = wasm.ContestedDocumentVotePollWinnerInfo.fromObject(obj);
      expect(restored.kind).to.equal(info.kind);
    });
  });
});
