import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import { identifier } from './mocks/Identity/index.js';

const testIdentifier2 = 'AV2kvu8V5epFNxXJSqSN1gXaqgrBL3cfSLafDH7w3fEP';

before(async () => {
  await initWasm();
});

describe('StateTransitionProofResult types', () => {
  describe('VerifiedTokenBalanceAbsence', () => {
    it('should round-trip through fromObject/toObject', () => {
      const data = { tokenId: identifier };
      const result = wasm.VerifiedTokenBalanceAbsence.fromObject(data);

      expect(result.tokenId.toBase58()).to.equal(identifier);

      // toObject serializes Identifier as Uint8Array (serde bytes)
      const expectedBytes = result.tokenId.toBytes();
      const obj = result.toObject();
      expect(obj.tokenId).to.deep.equal(expectedBytes);

      const roundtrip = wasm.VerifiedTokenBalanceAbsence.fromObject(obj);
      expect(roundtrip.tokenId.toBase58()).to.equal(identifier);
    });

    it('should round-trip through fromJSON/toJSON', () => {
      const data = { tokenId: identifier };
      const result = wasm.VerifiedTokenBalanceAbsence.fromJSON(data);

      // toJSON uses human-readable serde — Identifier stays as base58 string
      const json = result.toJSON();
      expect(json.tokenId).to.equal(identifier);

      const roundtrip = wasm.VerifiedTokenBalanceAbsence.fromJSON(json);
      expect(roundtrip.tokenId.toBase58()).to.equal(identifier);
    });
  });

  describe('VerifiedTokenBalance', () => {
    it('should round-trip through fromObject/toObject', () => {
      const data = { tokenId: identifier, balance: 500000n };
      const result = wasm.VerifiedTokenBalance.fromObject(data);

      expect(result.tokenId.toBase58()).to.equal(identifier);
      expect(result.balance).to.equal(500000n);

      const expectedBytes = result.tokenId.toBytes();
      const obj = result.toObject();
      expect(obj.tokenId).to.deep.equal(expectedBytes);
      expect(obj.balance).to.equal(500000n);

      const roundtrip = wasm.VerifiedTokenBalance.fromObject(obj);
      expect(roundtrip.tokenId.toBase58()).to.equal(identifier);
      expect(roundtrip.balance).to.equal(500000n);
    });
  });

  describe('VerifiedTokenGroupActionWithTokenBalance', () => {
    it('should round-trip with all fields', () => {
      const data = {
        groupPower: 42,
        actionStatus: 'ActionActive',
        balance: 100000n,
      };
      const result = wasm.VerifiedTokenGroupActionWithTokenBalance.fromObject(data);

      expect(result.groupPower).to.equal(42);
      expect(result.actionStatus).to.equal('ActionActive');
      expect(result.balance).to.equal(100000n);

      const obj = result.toObject();
      expect(obj.groupPower).to.equal(42);
      expect(obj.actionStatus).to.equal('ActionActive');

      const roundtrip = wasm.VerifiedTokenGroupActionWithTokenBalance.fromObject(obj);
      expect(roundtrip.groupPower).to.equal(42);
      expect(roundtrip.actionStatus).to.equal('ActionActive');
    });

    it('should round-trip with null balance', () => {
      const data = {
        groupPower: 10,
        actionStatus: 'ActionClosed',
        balance: null,
      };
      const result = wasm.VerifiedTokenGroupActionWithTokenBalance.fromObject(data);

      expect(result.groupPower).to.equal(10);
      expect(result.actionStatus).to.equal('ActionClosed');
      expect(result.balance).to.be.undefined();

      const obj = result.toObject();
      const roundtrip = wasm.VerifiedTokenGroupActionWithTokenBalance.fromObject(obj);
      expect(roundtrip.balance).to.be.undefined();
    });
  });

  // =========================================================================
  // Serde round-trip tests — types with Option<ComplexType> set to null
  // =========================================================================

  describe('VerifiedTokenPricingSchedule', () => {
    it('should round-trip with null pricingSchedule', () => {
      const data = { tokenId: identifier, pricingSchedule: null };
      const result = wasm.VerifiedTokenPricingSchedule.fromObject(data);

      expect(result.tokenId.toBase58()).to.equal(identifier);
      expect(result.pricingSchedule).to.be.undefined();

      const obj = result.toObject();
      const roundtrip = wasm.VerifiedTokenPricingSchedule.fromObject(obj);
      expect(roundtrip.tokenId.toBase58()).to.equal(identifier);
      expect(roundtrip.pricingSchedule).to.be.undefined();
    });
  });

  describe('VerifiedTokenGroupActionWithDocument', () => {
    it('should round-trip with null document', () => {
      const data = { groupPower: 10, document: null };
      const result = wasm.VerifiedTokenGroupActionWithDocument.fromObject(data);

      expect(result.groupPower).to.equal(10);
      expect(result.document).to.be.undefined();

      const obj = result.toObject();
      expect(obj.groupPower).to.equal(10);

      const roundtrip = wasm.VerifiedTokenGroupActionWithDocument.fromObject(obj);
      expect(roundtrip.groupPower).to.equal(10);
      expect(roundtrip.document).to.be.undefined();
    });
  });

  describe('VerifiedTokenGroupActionWithTokenIdentityInfo', () => {
    it('should round-trip with null tokenInfo', () => {
      const data = {
        groupPower: 5,
        actionStatus: 'ActionClosed',
        tokenInfo: null,
      };
      const result = wasm.VerifiedTokenGroupActionWithTokenIdentityInfo.fromObject(data);

      expect(result.groupPower).to.equal(5);
      expect(result.actionStatus).to.equal('ActionClosed');
      expect(result.tokenInfo).to.be.undefined();

      const obj = result.toObject();
      const roundtrip = wasm.VerifiedTokenGroupActionWithTokenIdentityInfo.fromObject(obj);
      expect(roundtrip.groupPower).to.equal(5);
      expect(roundtrip.actionStatus).to.equal('ActionClosed');
      expect(roundtrip.tokenInfo).to.be.undefined();
    });
  });

  describe('VerifiedTokenGroupActionWithTokenPricingSchedule', () => {
    it('should round-trip with null pricingSchedule', () => {
      const data = {
        groupPower: 3,
        actionStatus: 'ActionActive',
        pricingSchedule: null,
      };
      const result = wasm.VerifiedTokenGroupActionWithTokenPricingSchedule.fromObject(data);

      expect(result.groupPower).to.equal(3);
      expect(result.actionStatus).to.equal('ActionActive');
      expect(result.pricingSchedule).to.be.undefined();

      const obj = result.toObject();
      const roundtrip = wasm.VerifiedTokenGroupActionWithTokenPricingSchedule.fromObject(obj);
      expect(roundtrip.groupPower).to.equal(3);
      expect(roundtrip.actionStatus).to.equal('ActionActive');
      expect(roundtrip.pricingSchedule).to.be.undefined();
    });
  });

  // =========================================================================
  // Serde round-trip tests — types with versioned-enum inner types
  // =========================================================================

  describe('VerifiedTokenIdentityInfo', () => {
    it('should round-trip with V0 tokenInfo', () => {
      const data = {
        tokenId: identifier,
        // IdentityTokenInfo is an internally tagged enum: { $formatVersion: "0", frozen: bool }
        tokenInfo: { $formatVersion: "0", frozen: false },
      };
      const result = wasm.VerifiedTokenIdentityInfo.fromObject(data);

      expect(result.tokenId.toBase58()).to.equal(identifier);
      expect(result.tokenInfo).to.not.be.null();

      const obj = result.toObject();
      const roundtrip = wasm.VerifiedTokenIdentityInfo.fromObject(obj);
      expect(roundtrip.tokenId.toBase58()).to.equal(identifier);
    });
  });

  describe('VerifiedTokenStatus', () => {
    it('should round-trip with V0 tokenStatus', () => {
      const data = {
        // TokenStatus is an internally tagged enum: { $formatVersion: "0", paused: bool }
        tokenStatus: { $formatVersion: "0", paused: false },
      };
      const result = wasm.VerifiedTokenStatus.fromObject(data);

      expect(result.tokenStatus).to.not.be.null();

      const obj = result.toObject();
      const roundtrip = wasm.VerifiedTokenStatus.fromObject(obj);
      expect(roundtrip.tokenStatus).to.not.be.null();
    });
  });

  // =========================================================================
  // Serde round-trip tests — types with PartialIdentity
  // =========================================================================

  describe('VerifiedPartialIdentity', () => {
    it('should round-trip with minimal PartialIdentity', () => {
      // PartialIdentity.id is raw Identifier (expects bytes, not string)
      const idBytes = new wasm.Identifier(identifier).toBytes();
      const data = {
        partialIdentity: {
          id: idBytes,
          loadedPublicKeys: {},
          balance: null,
          revision: null,
          notFoundPublicKeys: [],
        },
      };
      const result = wasm.VerifiedPartialIdentity.fromObject(data);

      expect(result.partialIdentity).to.not.be.null();

      const obj = result.toObject();
      const roundtrip = wasm.VerifiedPartialIdentity.fromObject(obj);
      expect(roundtrip.partialIdentity).to.not.be.null();
    });
  });

  describe('VerifiedBalanceTransfer', () => {
    it('should round-trip with minimal sender and recipient', () => {
      // PartialIdentity.id is raw Identifier (expects bytes, not string)
      const idBytes1 = new wasm.Identifier(identifier).toBytes();
      const idBytes2 = new wasm.Identifier(testIdentifier2).toBytes();
      const partialId = (idBytes: Uint8Array) => ({
        id: idBytes,
        loadedPublicKeys: {},
        balance: null,
        revision: null,
        notFoundPublicKeys: [],
      });

      const data = {
        sender: partialId(idBytes1),
        recipient: partialId(idBytes2),
      };
      const result = wasm.VerifiedBalanceTransfer.fromObject(data);

      expect(result.sender).to.not.be.null();
      expect(result.recipient).to.not.be.null();

      const obj = result.toObject();
      const roundtrip = wasm.VerifiedBalanceTransfer.fromObject(obj);
      expect(roundtrip.sender).to.not.be.null();
      expect(roundtrip.recipient).to.not.be.null();
    });
  });

  // =========================================================================
  // Serde round-trip tests — types with Identity
  // =========================================================================

  describe('VerifiedIdentity', () => {
    it('should construct from object with minimal Identity', () => {
      // Identity uses #[serde(tag = "$formatVersion")] with V0 renamed to "0"
      // IdentityV0 uses #[serde(rename_all = "camelCase")]
      // Inside internally-tagged enum, Identifier expects base58 string (not bytes)
      const data = {
        identity: {
          $formatVersion: '0',
          id: identifier,
          publicKeys: [],
          balance: 0,
          revision: 0,
        },
      };
      const result = wasm.VerifiedIdentity.fromObject(data);

      expect(result.identity).to.not.be.null();

      const obj = result.toObject();
      expect(obj).to.have.property('identity');
    });
  });

  // =========================================================================
  // Serde round-trip tests — types with Vote
  // =========================================================================

  describe('VerifiedMasternodeVote', () => {
    it('should construct from object with Abstain vote', () => {
      // Vote:               internal tag `$type` ($-prefix because the level
      //                     also carries the inner `$formatVersion`)
      // ResourceVote:       single V0 variant flattens its body, contributing
      //                     `$formatVersion: "0"` at top level
      // VotePoll:           internal tag `type` (plain — no `$`-fields here)
      // ResourceVoteChoice: custom serde, flat `{type, identity?}`
      const data = {
        vote: {
          $type: 'resourceVote',
          $formatVersion: '0',
          votePoll: {
            $type: 'contestedDocumentResourceVotePoll',
            contractId: identifier,
            documentTypeName: 'domain',
            indexName: 'parentNameAndLabel',
            indexValues: ['dash', 'test'],
          },
          resourceVoteChoice: { $type: 'abstain' },
        },
      };
      const result = wasm.VerifiedMasternodeVote.fromObject(data);

      expect(result.vote).to.not.be.null();

      const obj = result.toObject();
      expect(obj).to.have.property('vote');
    });
  });

  describe('VerifiedNextDistribution', () => {
    it('should construct from object with Abstain vote', () => {
      const data = {
        vote: {
          $type: 'resourceVote',
          $formatVersion: '0',
          votePoll: {
            $type: 'contestedDocumentResourceVotePoll',
            contractId: identifier,
            documentTypeName: 'domain',
            indexName: 'parentNameAndLabel',
            indexValues: ['dash', 'test'],
          },
          resourceVoteChoice: { $type: 'abstain' },
        },
      };
      const result = wasm.VerifiedNextDistribution.fromObject(data);

      expect(result.vote).to.not.be.null();

      const obj = result.toObject();
      expect(obj).to.have.property('vote');
    });
  });

  // =========================================================================
  // Manual fromObject/fromJSON tests — Map-based types
  // =========================================================================

  describe('VerifiedTokenIdentitiesBalances', () => {
    it('should round-trip through fromObject/toObject with Map', () => {
      const id1 = new wasm.Identifier(identifier);
      const balancesMap = new Map();
      balancesMap.set(id1.toBase58(), 999000n);

      const data = { balances: balancesMap };
      const result = wasm.VerifiedTokenIdentitiesBalances.fromObject(data);

      expect(result.balances).to.be.instanceOf(Map);
      expect(result.balances.size).to.equal(1);

      const obj = result.toObject();
      expect(obj).to.have.property('balances');

      const roundtrip = wasm.VerifiedTokenIdentitiesBalances.fromObject(obj);
      expect(roundtrip.balances.size).to.equal(1);
    });

    it('should handle empty Map', () => {
      const data = { balances: new Map() };
      const result = wasm.VerifiedTokenIdentitiesBalances.fromObject(data);

      expect(result.balances.size).to.equal(0);
    });

    // Regression: toJSON() must produce a value where JSON.stringify preserves
    // Map entries. js_sys::Map embedded in a JsValue serialises to {} via
    // JSON.stringify, so toJSON() has to normalise the Map to a plain object.
    it('toJSON() should preserve Map entries through JSON.stringify', () => {
      const id1 = new wasm.Identifier(identifier);
      const balancesMap = new Map();
      balancesMap.set(id1.toBase58(), 999000n);

      const result = wasm.VerifiedTokenIdentitiesBalances.fromObject({ balances: balancesMap });
      const json = result.toJSON();

      const stringified = JSON.stringify(json);
      const parsed = JSON.parse(stringified);
      expect(parsed.balances).to.have.property(id1.toBase58());
      // BigInt is normalised to a string for JSON compatibility.
      expect(parsed.balances[id1.toBase58()]).to.equal('999000');
    });
  });

  describe('VerifiedDocuments', () => {
    it('should round-trip through fromObject/toObject with Map', () => {
      const id1 = new wasm.Identifier(identifier);
      const docsMap = new Map();
      // null values represent absent documents
      docsMap.set(id1.toBase58(), null);

      const data = { documents: docsMap };
      const result = wasm.VerifiedDocuments.fromObject(data);

      expect(result.documents).to.be.instanceOf(Map);
      expect(result.documents.size).to.equal(1);

      const obj = result.toObject();
      expect(obj).to.have.property('documents');

      const roundtrip = wasm.VerifiedDocuments.fromObject(obj);
      expect(roundtrip.documents.size).to.equal(1);
    });

    it('should handle empty Map', () => {
      const data = { documents: new Map() };
      const result = wasm.VerifiedDocuments.fromObject(data);

      expect(result.documents.size).to.equal(0);
    });

    it('toJSON() should preserve Map entries through JSON.stringify', () => {
      const id1 = new wasm.Identifier(identifier);
      const docsMap = new Map();
      docsMap.set(id1.toBase58(), null);

      const result = wasm.VerifiedDocuments.fromObject({ documents: docsMap });
      const parsed = JSON.parse(JSON.stringify(result.toJSON()));

      expect(parsed.documents).to.have.property(id1.toBase58());
      expect(parsed.documents[id1.toBase58()]).to.equal(null);
    });
  });

  describe('VerifiedAddressInfos', () => {
    it('should round-trip through fromObject/toObject with Map', () => {
      const infosMap = new Map();
      // hex key -> undefined value (absent address info)
      infosMap.set('abcdef0123456789', null);

      const data = { addressInfos: infosMap };
      const result = wasm.VerifiedAddressInfos.fromObject(data);

      expect(result.addressInfos).to.be.instanceOf(Map);
      expect(result.addressInfos.size).to.equal(1);

      const obj = result.toObject();
      expect(obj).to.have.property('addressInfos');

      const roundtrip = wasm.VerifiedAddressInfos.fromObject(obj);
      expect(roundtrip.addressInfos.size).to.equal(1);
    });

    it('should handle empty Map', () => {
      const data = { addressInfos: new Map() };
      const result = wasm.VerifiedAddressInfos.fromObject(data);

      expect(result.addressInfos.size).to.equal(0);
    });

    it('toJSON() should preserve Map entries through JSON.stringify', () => {
      const infosMap = new Map();
      infosMap.set('abcdef0123456789', null);

      const result = wasm.VerifiedAddressInfos.fromObject({ addressInfos: infosMap });
      const parsed = JSON.parse(JSON.stringify(result.toJSON()));

      expect(parsed.addressInfos).to.have.property('abcdef0123456789');
      expect(parsed.addressInfos.abcdef0123456789).to.equal(null);
    });
  });

  // =========================================================================
  // Manual fromObject/fromJSON tests — composite types
  // =========================================================================

  describe('VerifiedIdentityFullWithAddressInfos', () => {
    it('should construct from object with Identity + addressInfos', () => {
      // Identity uses internally-tagged enum -> Identifier expects base58 string
      const identityData = {
        $formatVersion: '0',
        id: identifier,
        publicKeys: [],
        balance: 0,
        revision: 0,
      };
      const infosMap = new Map();
      infosMap.set('deadbeef', null);

      const data = { identity: identityData, addressInfos: infosMap };
      const result = wasm.VerifiedIdentityFullWithAddressInfos.fromObject(data);

      expect(result.identity).to.not.be.null();
      expect(result.addressInfos).to.be.instanceOf(Map);
      expect(result.addressInfos.size).to.equal(1);

      const obj = result.toObject();
      expect(obj).to.have.property('identity');
      expect(obj).to.have.property('addressInfos');
    });

    it('toJSON() should preserve Map entries through JSON.stringify', () => {
      const identityData = {
        $formatVersion: '0',
        id: identifier,
        publicKeys: [],
        balance: 0,
        revision: 0,
      };
      const infosMap = new Map();
      infosMap.set('deadbeef', null);

      const result = wasm.VerifiedIdentityFullWithAddressInfos.fromObject({
        identity: identityData,
        addressInfos: infosMap,
      });
      const parsed = JSON.parse(JSON.stringify(result.toJSON()));

      expect(parsed.addressInfos).to.have.property('deadbeef');
      expect(parsed.addressInfos.deadbeef).to.equal(null);
    });
  });

  describe('VerifiedIdentityWithAddressInfos', () => {
    it('should construct from object with PartialIdentity + addressInfos', () => {
      // PartialIdentity.id is raw Identifier (expects bytes, not string)
      const idBytes = new wasm.Identifier(identifier).toBytes();
      const piData = {
        id: idBytes,
        loadedPublicKeys: {},
        balance: null,
        revision: null,
        notFoundPublicKeys: [],
      };
      const infosMap = new Map();
      infosMap.set('cafebabe', null);

      const data = { partialIdentity: piData, addressInfos: infosMap };
      const result = wasm.VerifiedIdentityWithAddressInfos.fromObject(data);

      expect(result.partialIdentity).to.not.be.null();
      expect(result.addressInfos).to.be.instanceOf(Map);
      expect(result.addressInfos.size).to.equal(1);

      const obj = result.toObject();
      expect(obj).to.have.property('partialIdentity');
      expect(obj).to.have.property('addressInfos');
    });

    it('toJSON() should preserve Map entries through JSON.stringify', () => {
      const idBytes = new wasm.Identifier(identifier).toBytes();
      const piData = {
        id: idBytes,
        loadedPublicKeys: {},
        balance: null,
        revision: null,
        notFoundPublicKeys: [],
      };
      const infosMap = new Map();
      infosMap.set('cafebabe', null);

      const result = wasm.VerifiedIdentityWithAddressInfos.fromObject({
        partialIdentity: piData,
        addressInfos: infosMap,
      });
      const parsed = JSON.parse(JSON.stringify(result.toJSON()));

      expect(parsed.addressInfos).to.have.property('cafebabe');
      expect(parsed.addressInfos.cafebabe).to.equal(null);
    });
  });
});
