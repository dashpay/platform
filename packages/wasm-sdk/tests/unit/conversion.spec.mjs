import init, * as sdk from '../../dist/sdk.compressed.js';

describe('serde conversions (unit)', () => {
  before(async () => {
    await init();
  });

  describe('ProofMetadataResponse BigInt serialization', () => {
    it('should serialize BigInt data to JSON as string', () => {
      const chainId = new Uint8Array([1, 2, 3, 4]);
      const metadata = new sdk.ResponseMetadata(1n, 2, 3, 4n, 5, chainId);
      const proof = new sdk.ProofInfo(
        new Uint8Array([1, 2, 3]),
        new Uint8Array([4, 5, 6]),
        new Uint8Array([7, 8, 9]),
        1,
        new Uint8Array([10, 11, 12]),
        100,
      );

      // Create ProofMetadataResponse with BigInt data (simulating large credit balance)
      const largeBigInt = 23522425453263151n; // Value > Number.MAX_SAFE_INTEGER
      const response = new sdk.ProofMetadataResponse(largeBigInt, metadata, proof);

      // toJSON should not throw and should convert BigInt to string
      const json = response.toJSON();
      expect(json).to.have.property('data');
      expect(json).to.have.property('metadata');
      expect(json).to.have.property('proof');

      // BigInt should be serialized as string
      expect(json.data).to.equal('23522425453263151');
    });

    it('should serialize object with BigInt properties to JSON', () => {
      const chainId = new Uint8Array([1, 2, 3, 4]);
      const metadata = new sdk.ResponseMetadata(1n, 2, 3, 4n, 5, chainId);
      const proof = new sdk.ProofInfo(
        new Uint8Array([1, 2, 3]),
        new Uint8Array([4, 5, 6]),
        new Uint8Array([7, 8, 9]),
        1,
        new Uint8Array([10, 11, 12]),
        100,
      );

      // Create ProofMetadataResponse with object containing BigInt
      const dataWithBigInt = {
        totalCredits: 23522425453263151n,
        count: 42,
        nested: {
          balance: 9007199254740992n, // Exactly MAX_SAFE_INTEGER + 1
        },
      };
      const response = new sdk.ProofMetadataResponse(dataWithBigInt, metadata, proof);

      // toJSON should not throw
      const json = response.toJSON();
      expect(json.data.totalCredits).to.equal('23522425453263151');
      expect(json.data.count).to.equal(42);
      expect(json.data.nested.balance).to.equal('9007199254740992');
    });

    it('should serialize array with BigInt values to JSON', () => {
      const chainId = new Uint8Array([1, 2, 3, 4]);
      const metadata = new sdk.ResponseMetadata(1n, 2, 3, 4n, 5, chainId);
      const proof = new sdk.ProofInfo(
        new Uint8Array([1, 2, 3]),
        new Uint8Array([4, 5, 6]),
        new Uint8Array([7, 8, 9]),
        1,
        new Uint8Array([10, 11, 12]),
        100,
      );

      // Create ProofMetadataResponse with array containing BigInt
      const dataWithBigIntArray = [1n, 2n, 23522425453263151n];
      const response = new sdk.ProofMetadataResponse(dataWithBigIntArray, metadata, proof);

      // toJSON should not throw
      const json = response.toJSON();
      expect(json.data).to.deep.equal(['1', '2', '23522425453263151']);
    });
  });

  it('ResponseMetadata: getter returns Uint8Array, toJSON returns base64', () => {
    const chainId = new Uint8Array([1, 2, 3, 4]);
    const meta = new sdk.ResponseMetadata(1n, 2, 3, 4n, 5, chainId);

    const chainFromGetter = meta.chainId;
    expect(chainFromGetter).to.be.instanceOf(Uint8Array);
    expect([...chainFromGetter]).to.deep.equal([...chainId]);

    const json = meta.toJSON();
    expect(json.chainId).to.be.a('string');
    expect(json.chainId).to.equal(Buffer.from(chainId).toString('base64'));

    const metaFromJson = sdk.ResponseMetadata.fromJSON(json);
    expect([...metaFromJson.chainId]).to.deep.equal([...chainId]);
  });

  describe('ProofMetadataResponse with Identity data', () => {
    it('should serialize Identity as data with full JSON including metadata, proof, and identity', () => {
      // Create metadata and proof
      const chainId = new Uint8Array([1, 2, 3, 4]);
      const metadata = new sdk.ResponseMetadata(100n, 50, 10, 1234567890n, 1, chainId);
      const proof = new sdk.ProofInfo(
        new Uint8Array([10, 20, 30]), // grovedbProof
        new Uint8Array([40, 50, 60]), // quorumHash
        new Uint8Array([70, 80, 90]), // signature
        5, // round
        new Uint8Array([100, 110, 120]), // blockIdHash
        200, // quorumType
      );

      // Create an Identity
      const identifierId = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';
      const identity = new sdk.Identity(identifierId);
      identity.balance = 1000000000n; // 10 DASH in credits
      identity.revision = 5n;

      // Create ProofMetadataResponse with Identity.toJSON() as data
      // Using toJSON() ensures all nested objects (like Identifier)
      // are converted to JSON-safe values
      const identityJson = identity.toJSON();
      const response = new sdk.ProofMetadataResponse(identityJson, metadata, proof);

      // Call toJSON - should produce full JSON representation
      const json = response.toJSON();

      // Verify structure
      expect(json).to.have.property('data');
      expect(json).to.have.property('metadata');
      expect(json).to.have.property('proof');

      // Verify metadata
      expect(json.metadata.height).to.equal(100);
      expect(json.metadata.coreChainLockedHeight).to.equal(50);
      expect(json.metadata.epoch).to.equal(10);
      expect(json.metadata.timeMs).to.equal(1234567890);
      expect(json.metadata.protocolVersion).to.equal(1);
      expect(json.metadata.chainId).to.equal(Buffer.from(chainId).toString('base64'));

      // Verify proof
      expect(json.proof.grovedbProof).to.equal(Buffer.from([10, 20, 30]).toString('base64'));
      expect(json.proof.quorumHash).to.equal(Buffer.from([40, 50, 60]).toString('base64'));
      expect(json.proof.signature).to.equal(Buffer.from([70, 80, 90]).toString('base64'));
      expect(json.proof.round).to.equal(5);
      expect(json.proof.blockIdHash).to.equal(Buffer.from([100, 110, 120]).toString('base64'));
      expect(json.proof.quorumType).to.equal(200);

      // Verify identity data - id should be Base58 in JSON
      expect(json.data).to.have.property('id');
      expect(json.data.id).to.equal(identifierId);
      // Values within safe integer range are numbers, large values would be strings
      expect(json.data.balance).to.equal(1000000000);
      expect(json.data.revision).to.equal(5);
      expect(json.data.publicKeys).to.be.an('array');
    });

    it('should round-trip ProofMetadataResponse with Identity data through JSON', () => {
      const chainId = new Uint8Array([5, 6, 7, 8]);
      const metadata = new sdk.ResponseMetadata(200n, 100, 20, 9876543210n, 2, chainId);
      const proof = new sdk.ProofInfo(
        new Uint8Array([1, 2, 3, 4, 5]),
        new Uint8Array([6, 7, 8, 9, 10]),
        new Uint8Array([11, 12, 13, 14, 15]),
        10,
        new Uint8Array([16, 17, 18, 19, 20]),
        300,
      );

      const identifierId = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';
      const identity = new sdk.Identity(identifierId);
      identity.balance = 5000000000n;
      identity.revision = 10n;

      // Use toJSON() to get JSON-safe representation of identity
      const response = new sdk.ProofMetadataResponse(identity.toJSON(), metadata, proof);
      const json = response.toJSON();

      // Round-trip through JSON
      const restored = sdk.ProofMetadataResponse.fromJSON(json);
      const restoredJson = restored.toJSON();

      // Verify JSON matches
      expect(restoredJson).to.deep.equal(json);

      // Verify we can access the data
      expect(restored.data.id).to.equal(identifierId);
      // metadata.height getter returns BigInt
      expect(restored.metadata.height).to.equal(200n);
      expect(restored.proof.round).to.equal(10);
    });
  });

  it('Identifier-backed structs: toObject returns bytes, toJSON returns Base58', () => {
    const bytes = new Uint8Array(32).fill(7);
    const expectedBase58 = sdk.Identifier.fromBytes(Array.from(bytes)).toBase58();
    const identifier = sdk.Identifier.fromBase58(expectedBase58);
    const identifier2 = sdk.Identifier.fromBase58(expectedBase58);
    const info = new sdk.DpnsUsernameInfo('alice', identifier, identifier2);

    const identityBytes = info.identityId.toBytes();
    const documentBytes = info.documentId.toBytes();
    expect(identityBytes.length).to.equal(32);
    expect(documentBytes.length).to.equal(32);
    expect(Buffer.from(identityBytes)).to.deep.equal(Buffer.from(bytes));
    expect(Buffer.from(documentBytes)).to.deep.equal(Buffer.from(bytes));

    const object = info.toObject();
    expect(Buffer.from(object.identityId)).to.deep.equal(Buffer.from(bytes));
    expect(Buffer.from(object.documentId)).to.deep.equal(Buffer.from(bytes));

    const json = info.toJSON();
    expect(json.identityId).to.equal(expectedBase58);
    expect(json.documentId).to.equal(expectedBase58);

    const roundtrip = sdk.DpnsUsernameInfo.fromJSON(json);
    expect(roundtrip.identityId.toBase58()).to.equal(expectedBase58);
    expect(roundtrip.documentId.toBase58()).to.equal(expectedBase58);
  });
});
