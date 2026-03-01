import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('GroupAction', () => {
  const contractIdBase58 = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';
  const proposerIdBase58 = '2QjL594djCH2NyDsn45vd6yQjEDHupMKo7CEGVTHtQxU';
  // Mint(amount, recipientId, publicNote)
  const recipientIdBase58 = '4fJLR2GYTPFdomuTVvNy3VRrvWgvkKPzqehEBpNf2nk6';

  // JSON fixture for a GroupAction V0 containing a TokenEvent::Mint
  const jsonFixture = {
    $formatVersion: '0',
    contract_id: contractIdBase58,
    proposer_id: proposerIdBase58,
    token_contract_position: 0,
    event: {
      TokenEvent: {
        Mint: [1000, recipientIdBase58, 'test mint note'],
      },
    },
  };

  describe('fromJSON()', () => {
    it('should create from JSON and verify getters', () => {
      const action = wasm.GroupAction.fromJSON(jsonFixture);

      expect(action.contractId.toBase58()).to.equal(contractIdBase58);
      expect(action.proposerId.toBase58()).to.equal(proposerIdBase58);
      expect(action.tokenContractPosition).to.equal(0);
    });

    it('should expose event getter as GroupActionEvent', () => {
      const action = wasm.GroupAction.fromJSON(jsonFixture);
      const event = action.event;

      expect(event).to.be.instanceOf(wasm.GroupActionEvent);
      expect(event.variant).to.equal(wasm.GroupActionEventVariant.TokenEvent);
    });
  });

  describe('toJSON()', () => {
    it('should round-trip via fromJSON/toJSON', () => {
      const action = wasm.GroupAction.fromJSON(jsonFixture);
      const json = action.toJSON();
      const restored = wasm.GroupAction.fromJSON(json);

      expect(restored.contractId.toBase58()).to.equal(contractIdBase58);
      expect(restored.proposerId.toBase58()).to.equal(proposerIdBase58);
      expect(restored.tokenContractPosition).to.equal(0);
    });
  });

  describe('toObject()', () => {
    it('should serialize to Object', () => {
      const action = wasm.GroupAction.fromJSON(jsonFixture);
      const obj = action.toObject();

      expect(obj).to.be.an('object');
    });
  });

  describe('fromObject()', () => {
    it('should round-trip via toObject/fromObject', () => {
      const action = wasm.GroupAction.fromJSON(jsonFixture);
      const obj = action.toObject();
      const restored = wasm.GroupAction.fromObject(obj);

      expect(restored.contractId.toBase58()).to.equal(contractIdBase58);
      expect(restored.proposerId.toBase58()).to.equal(proposerIdBase58);
      expect(restored.tokenContractPosition).to.equal(0);
    });
  });

  describe('__type', () => {
    it('should return correct __type', () => {
      const action = wasm.GroupAction.fromJSON(jsonFixture);
      expect(action.__type).to.equal('GroupAction');
    });
  });
});

describe('GroupActionEvent', () => {
  const recipientIdBase58 = '4fJLR2GYTPFdomuTVvNy3VRrvWgvkKPzqehEBpNf2nk6';

  // TokenEvent::Freeze(frozenIdentifier, publicNote)
  const freezeEventFixture = {
    TokenEvent: {
      Freeze: [recipientIdBase58, 'freeze note'],
    },
  };

  // TokenEvent::Mint(amount, recipientId, publicNote)
  const mintEventFixture = {
    TokenEvent: {
      Mint: [500, recipientIdBase58, null],
    },
  };

  describe('fromJSON()', () => {
    it('should create from JSON and verify variant', () => {
      const event = wasm.GroupActionEvent.fromJSON(freezeEventFixture);

      expect(event.variant).to.equal(wasm.GroupActionEventVariant.TokenEvent);
    });
  });

  describe('tokenEvent()', () => {
    it('should return TokenEvent', () => {
      const event = wasm.GroupActionEvent.fromJSON(mintEventFixture);
      const tokenEvent = event.tokenEvent();

      expect(tokenEvent).to.be.instanceOf(wasm.TokenEvent);
    });
  });

  describe('eventName()', () => {
    it('should return event name', () => {
      const event = wasm.GroupActionEvent.fromJSON(freezeEventFixture);
      const name = event.eventName();

      expect(name).to.be.a('string');
      expect(name).to.include('Token');
    });
  });

  describe('publicNote()', () => {
    it('should return public note when present', () => {
      const event = wasm.GroupActionEvent.fromJSON(freezeEventFixture);
      expect(event.publicNote()).to.equal('freeze note');
    });

    it('should return undefined when no public note', () => {
      const event = wasm.GroupActionEvent.fromJSON(mintEventFixture);
      expect(event.publicNote()).to.be.undefined();
    });
  });

  describe('toJSON()/fromJSON() round-trip', () => {
    it('should round-trip', () => {
      const event = wasm.GroupActionEvent.fromJSON(freezeEventFixture);
      const json = event.toJSON();
      const restored = wasm.GroupActionEvent.fromJSON(json);

      expect(restored.variant).to.equal(wasm.GroupActionEventVariant.TokenEvent);
      expect(restored.publicNote()).to.equal('freeze note');
    });
  });

  describe('toObject()/fromObject() round-trip', () => {
    it('should round-trip', () => {
      const event = wasm.GroupActionEvent.fromJSON(freezeEventFixture);
      const obj = event.toObject();
      const restored = wasm.GroupActionEvent.fromObject(obj);

      expect(restored.variant).to.equal(wasm.GroupActionEventVariant.TokenEvent);
    });
  });

  describe('__type', () => {
    it('should return correct __type', () => {
      const event = wasm.GroupActionEvent.fromJSON(mintEventFixture);
      expect(event.__type).to.equal('GroupActionEvent');
    });
  });
});

describe('TokenEvent', () => {
  const recipientIdBase58 = '4fJLR2GYTPFdomuTVvNy3VRrvWgvkKPzqehEBpNf2nk6';

  // TokenEvent::Mint(amount, recipientId, publicNote)
  const mintFixture = {
    Mint: [1000, recipientIdBase58, 'mint note'],
  };

  // TokenEvent::Burn(amount, burnFromId, publicNote)
  const burnFixture = {
    Burn: [500, recipientIdBase58, null],
  };

  // TokenEvent::Freeze(frozenId, publicNote)
  const freezeFixture = {
    Freeze: [recipientIdBase58, 'frozen'],
  };

  describe('fromJSON()', () => {
    it('should create Mint variant from JSON', () => {
      const event = wasm.TokenEvent.fromJSON(mintFixture);
      expect(event.variant).to.equal(wasm.TokenEventVariant.Mint);
    });

    it('should create Burn variant from JSON', () => {
      const event = wasm.TokenEvent.fromJSON(burnFixture);
      expect(event.variant).to.equal(wasm.TokenEventVariant.Burn);
    });

    it('should create Freeze variant from JSON', () => {
      const event = wasm.TokenEvent.fromJSON(freezeFixture);
      expect(event.variant).to.equal(wasm.TokenEventVariant.Freeze);
    });
  });

  describe('toJSON()/fromJSON() round-trip', () => {
    it('should round-trip Mint', () => {
      const event = wasm.TokenEvent.fromJSON(mintFixture);
      const json = event.toJSON();
      const restored = wasm.TokenEvent.fromJSON(json);

      expect(restored.variant).to.equal(wasm.TokenEventVariant.Mint);
    });

    it('should round-trip Freeze', () => {
      const event = wasm.TokenEvent.fromJSON(freezeFixture);
      const json = event.toJSON();
      const restored = wasm.TokenEvent.fromJSON(json);

      expect(restored.variant).to.equal(wasm.TokenEventVariant.Freeze);
    });
  });

  describe('toObject()/fromObject() round-trip', () => {
    it('should round-trip', () => {
      const event = wasm.TokenEvent.fromJSON(mintFixture);
      const obj = event.toObject();
      const restored = wasm.TokenEvent.fromObject(obj);

      expect(restored.variant).to.equal(wasm.TokenEventVariant.Mint);
    });
  });

  describe('__type', () => {
    it('should return correct __type', () => {
      const event = wasm.TokenEvent.fromJSON(mintFixture);
      expect(event.__type).to.equal('TokenEvent');
    });
  });
});
