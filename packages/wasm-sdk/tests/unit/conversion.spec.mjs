import init, * as sdk from '../../dist/sdk.compressed.js';

describe('serde conversions (unit)', () => {
  before(async () => {
    await init();
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
