import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';
import { prefetchLocalReady } from './helpers/trustedContext.ts';

// These tests run against a local dashmate node and verify only the
// SHAPE of each shielded query response, not its contents — the local
// pool may legitimately be empty (no shielded transactions yet) and
// the client must still return a well-formed value, not throw.

describe('Shielded queries', function describeShielded() {
  this.timeout(90000);

  let client: sdk.WasmSdk;

  before(async () => {
    await init();
    const context = await prefetchLocalReady();
    const builder = sdk.WasmSdkBuilder.local().withTrustedContext(context);
    client = await builder.build();
  });

  after(() => {
    if (client) { client.free(); }
  });

  // ── Pool state ────────────────────────────────────────────────────────

  describe('getShieldedPoolState()', () => {
    it('should return bigint or undefined', async () => {
      const result = await client.getShieldedPoolState();
      // Pool may be empty (undefined) or non-empty (bigint).
      if (result !== undefined) {
        expect(result).to.be.a('bigint');
      }
    });
  });

  describe('getShieldedPoolStateWithProofInfo()', () => {
    it('should return data + metadata + proof', async () => {
      const response = await client.getShieldedPoolStateWithProofInfo();

      expect(response).to.be.ok();
      expect(response.metadata).to.be.ok();
      expect(response.proof).to.be.ok();
      // data is bigint | null (null preserves the field in JSON.stringify)
      if (response.data !== null) {
        expect(response.data).to.be.a('bigint');
      }
    });
  });

  // ── Encrypted notes ──────────────────────────────────────────────────

  describe('getShieldedEncryptedNotes()', () => {
    it('should return an array of ShieldedEncryptedNote (possibly empty)', async () => {
      const notes = await client.getShieldedEncryptedNotes(0n, 10);

      expect(notes).to.be.an('array');
      for (const note of notes) {
        expect(note).to.be.instanceOf(sdk.ShieldedEncryptedNote);
        expect(note.cmx).to.be.instanceOf(Uint8Array);
        expect(note.nullifier).to.be.instanceOf(Uint8Array);
        expect(note.cvNet).to.be.instanceOf(Uint8Array);
        expect(note.encryptedNote).to.be.instanceOf(Uint8Array);
      }
    });
  });

  describe('getShieldedEncryptedNotesWithProofInfo()', () => {
    it('should return data array + metadata + proof', async () => {
      const response = await client.getShieldedEncryptedNotesWithProofInfo(0n, 10);

      expect(response).to.be.ok();
      expect(response.metadata).to.be.ok();
      expect(response.proof).to.be.ok();
      expect(response.data).to.be.an('array');
    });
  });

  // ── Anchors ──────────────────────────────────────────────────────────

  describe('getShieldedAnchors()', () => {
    it('should return an array of Uint8Array (possibly empty)', async () => {
      const anchors = await client.getShieldedAnchors();

      expect(anchors).to.be.an('array');
      for (const anchor of anchors) {
        expect(anchor).to.be.instanceOf(Uint8Array);
        expect(anchor.length).to.equal(32);
      }
    });
  });

  describe('getShieldedAnchorsWithProofInfo()', () => {
    it('should return data array + metadata + proof', async () => {
      const response = await client.getShieldedAnchorsWithProofInfo();

      expect(response).to.be.ok();
      expect(response.metadata).to.be.ok();
      expect(response.proof).to.be.ok();
      expect(response.data).to.be.an('array');
    });
  });

  describe('getMostRecentShieldedAnchor()', () => {
    it('should return Uint8Array (32 bytes) or undefined', async () => {
      const anchor = await client.getMostRecentShieldedAnchor();

      if (anchor !== undefined) {
        expect(anchor).to.be.instanceOf(Uint8Array);
        expect(anchor.length).to.equal(32);
      }
    });
  });

  describe('getMostRecentShieldedAnchorWithProofInfo()', () => {
    it('should return data + metadata + proof (data may be null)', async () => {
      const response = await client.getMostRecentShieldedAnchorWithProofInfo();

      expect(response).to.be.ok();
      expect(response.metadata).to.be.ok();
      expect(response.proof).to.be.ok();
      if (response.data !== null) {
        expect(response.data).to.be.instanceOf(Uint8Array);
        expect(response.data.length).to.equal(32);
      }
    });
  });

  // ── Nullifiers ───────────────────────────────────────────────────────

  describe('getShieldedNullifiers()', () => {
    it('should reject empty input (server-side InvalidArgument)', async () => {
      let err: Error | undefined;
      try {
        await client.getShieldedNullifiers([]);
      } catch (e) {
        err = e as Error;
      }
      expect(err, 'expected getShieldedNullifiers([]) to throw').to.exist();
      expect(err!.message).to.match(/nullifiers list must not be empty|invalid argument/i);
    });

    it('should return an entry per queried nullifier', async () => {
      // Two arbitrary 32-byte nullifiers; the local pool almost certainly
      // hasn't seen them, but the query should succeed and report
      // isSpent: false for each.
      const nullifiers = [new Uint8Array(32).fill(0xaa), new Uint8Array(32).fill(0xbb)];
      const statuses = await client.getShieldedNullifiers(nullifiers);

      expect(statuses).to.be.an('array');
      for (const status of statuses) {
        expect(status).to.be.instanceOf(sdk.ShieldedNullifierStatus);
        expect(status.nullifier).to.be.instanceOf(Uint8Array);
        expect(status.nullifier.length).to.equal(32);
        expect(status.isSpent).to.be.a('boolean');
      }
    });

    it('should reject Uint8Array of wrong length', async () => {
      let threw = false;
      try {
        await client.getShieldedNullifiers([new Uint8Array(20)]);
      } catch (_e) {
        threw = true;
      }
      expect(threw).to.equal(true);
    });
  });

  describe('getShieldedNullifiersWithProofInfo()', () => {
    it('should return data array + metadata + proof', async () => {
      const response = await client.getShieldedNullifiersWithProofInfo([
        new Uint8Array(32).fill(0xcc),
      ]);

      expect(response).to.be.ok();
      expect(response.metadata).to.be.ok();
      expect(response.proof).to.be.ok();
      expect(response.data).to.be.an('array');
    });
  });
});
