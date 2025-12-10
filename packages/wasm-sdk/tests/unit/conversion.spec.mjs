import init, * as sdk from '../../dist/sdk.compressed.js';

describe('serde conversions (unit)', () => {
  before(async () => {
    await init();
  });

  describe('ProofMetadataResponse BigInt serialization', () => {
    it('should serialize BigInt data to JSON as string', function () {
      const chainId = new Uint8Array([1, 2, 3, 4]);
      const metadata = new sdk.ResponseMetadata(1n, 2, 3, 4n, 5, chainId);
      const proof = new sdk.ProofInfo(
        new Uint8Array([1, 2, 3]),
        new Uint8Array([4, 5, 6]),
        new Uint8Array([7, 8, 9]),
        1,
        new Uint8Array([10, 11, 12]),
        100
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

    it('should serialize object with BigInt properties to JSON', function () {
      const chainId = new Uint8Array([1, 2, 3, 4]);
      const metadata = new sdk.ResponseMetadata(1n, 2, 3, 4n, 5, chainId);
      const proof = new sdk.ProofInfo(
        new Uint8Array([1, 2, 3]),
        new Uint8Array([4, 5, 6]),
        new Uint8Array([7, 8, 9]),
        1,
        new Uint8Array([10, 11, 12]),
        100
      );

      // Create ProofMetadataResponse with object containing BigInt
      const dataWithBigInt = {
        totalCredits: 23522425453263151n,
        count: 42,
        nested: {
          balance: 9007199254740992n // Exactly MAX_SAFE_INTEGER + 1
        }
      };
      const response = new sdk.ProofMetadataResponse(dataWithBigInt, metadata, proof);

      // toJSON should not throw
      const json = response.toJSON();
      expect(json.data.totalCredits).to.equal('23522425453263151');
      expect(json.data.count).to.equal(42);
      expect(json.data.nested.balance).to.equal('9007199254740992');
    });

    it('should serialize array with BigInt values to JSON', function () {
      const chainId = new Uint8Array([1, 2, 3, 4]);
      const metadata = new sdk.ResponseMetadata(1n, 2, 3, 4n, 5, chainId);
      const proof = new sdk.ProofInfo(
        new Uint8Array([1, 2, 3]),
        new Uint8Array([4, 5, 6]),
        new Uint8Array([7, 8, 9]),
        1,
        new Uint8Array([10, 11, 12]),
        100
      );

      // Create ProofMetadataResponse with array containing BigInt
      const dataWithBigIntArray = [1n, 2n, 23522425453263151n];
      const response = new sdk.ProofMetadataResponse(dataWithBigIntArray, metadata, proof);

      // toJSON should not throw
      const json = response.toJSON();
      expect(json.data).to.deep.equal(['1', '2', '23522425453263151']);
    });
  });

  it('ResponseMetadata: getter returns Uint8Array, toJSON returns base64', function () {
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

  it('Identifier-backed structs: toObject returns bytes, toJSON returns Base58', function () {
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
