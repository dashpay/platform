const assert = require('node:assert/strict');
const nodeMessages = require('../../clients/platform/v0/nodejs/platform_protoc');
const webMessages = require('../../clients/platform/v0/web/platform_pb');
const protobufMessages = require('../../clients/platform/v0/nodejs/platform_pbjs');
const driveMessages = require('../../clients/drive/v0/nodejs/drive_pbjs');

const name = 'GetIdentityByNonUniquePublicKeyHashResponse';
// Response V0 (tag 1) with result field 2: the index proof and the identity proof bytes,
// exactly as 4.1 nodes send a match.
const v0Wire = Buffer.from([
  0x0a, 0x0c, 0x12, 0x0a, 0x0a, 0x04, 0x0a, 0x02, 1, 2, 0x12, 0x02, 3, 4,
]);
// Response V1 (tag 2) with result field 2: one Proof.
const v1Wire = Buffer.from([0x12, 0x06, 0x12, 0x04, 0x0a, 0x02, 1, 2]);
// Response V0 with a Proof at field 4, as the 5.0 prereleases sent it.
const field4Wire = Buffer.from([0x0a, 0x06, 0x22, 0x04, 0x0a, 0x02, 1, 2]);

describe('non-unique identity proof wire format', () => {
  it('should round trip the two-proof V0 answer across all published JS clients', () => {
    const v0Name = `${name}V0`;
    const proof = new nodeMessages.Proof();
    proof.setGrovedbProof(Buffer.from([1, 2]));
    const proved = new nodeMessages[name][v0Name].IdentityProvedResponse();
    proved.setGrovedbIdentityPublicKeyHashProof(proof);
    proved.setIdentityProofBytes(Buffer.from([3, 4]));
    const v0 = new nodeMessages[name][v0Name]();
    v0.setProof(proved);
    const response = new nodeMessages[name]();
    response.setV0(v0);
    assert.deepEqual(Buffer.from(response.serializeBinary()), v0Wire);

    const web = webMessages[name].deserializeBinary(v0Wire).getV0().getProof();
    assert.deepEqual(
      Buffer.from(web.getGrovedbIdentityPublicKeyHashProof().getGrovedbProof_asU8()),
      Buffer.from([1, 2]),
    );
    assert.deepEqual(Buffer.from(web.getIdentityProofBytes_asU8()), Buffer.from([3, 4]));
    [protobufMessages, driveMessages].forEach((messages) => {
      const Response = messages.org.dash.platform.dapi.v0[name];
      const decoded = Response.decode(v0Wire);
      assert.deepEqual(
        Buffer.from(decoded.v0.proof.grovedbIdentityPublicKeyHashProof.grovedbProof),
        Buffer.from([1, 2]),
      );
      assert.deepEqual(Buffer.from(decoded.v0.proof.identityProofBytes), Buffer.from([3, 4]));
      assert.deepEqual(Buffer.from(Response.encode(decoded).finish()), v0Wire);
    });
  });

  it('should round trip the single-proof V1 answer across all published JS clients', () => {
    const proof = new nodeMessages.Proof();
    proof.setGrovedbProof(Buffer.from([1, 2]));
    const v1 = new nodeMessages[name][`${name}V1`]();
    v1.setProof(proof);
    const response = new nodeMessages[name]();
    response.setV1(v1);
    assert.deepEqual(Buffer.from(response.serializeBinary()), v1Wire);

    const web = webMessages[name].deserializeBinary(v1Wire);
    assert.equal(web.hasV0(), false);
    assert.deepEqual(
      Buffer.from(web.getV1().getProof().getGrovedbProof_asU8()),
      Buffer.from([1, 2]),
    );
    [protobufMessages, driveMessages].forEach((messages) => {
      const Response = messages.org.dash.platform.dapi.v0[name];
      const decoded = Response.decode(v1Wire);
      assert.equal(decoded.v0, null);
      assert.deepEqual(Buffer.from(decoded.v1.proof.grovedbProof), Buffer.from([1, 2]));
      assert.deepEqual(Buffer.from(Response.encode(decoded).finish()), v1Wire);
    });
  });

  it('should find no result in a V0 answer at field 4', () => {
    [nodeMessages, webMessages].forEach((messages) => {
      const v0 = messages[name].deserializeBinary(field4Wire).getV0();
      assert.equal(v0.getResultCase(), messages[name][`${name}V0`].ResultCase.RESULT_NOT_SET);
    });
    [protobufMessages, driveMessages].forEach((messages) => {
      const { v0 } = messages.org.dash.platform.dapi.v0[name].decode(field4Wire);
      assert.equal(v0.result, undefined);
      assert.equal(v0.proof, null);
      assert.equal(v0.identity, null);
    });
  });
});
