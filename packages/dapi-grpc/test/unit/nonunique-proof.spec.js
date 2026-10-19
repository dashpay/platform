const assert = require('node:assert/strict');
const nodeMessages = require('../../clients/platform/v0/nodejs/platform_protoc');
const webMessages = require('../../clients/platform/v0/web/platform_pb');
const protobufMessages = require('../../clients/platform/v0/nodejs/platform_pbjs');
const driveMessages = require('../../clients/drive/v0/nodejs/drive_pbjs');

const name = 'GetIdentityByNonUniquePublicKeyHashResponse';
const innerName = `${name}V0`;
const singleWire = Buffer.from([0x0a, 0x06, 0x22, 0x04, 0x0a, 0x02, 1, 2]);
const retiredWire = Buffer.from([0x0a, 0x08, 0x12, 0x06, 0x0a, 0x04, 0x0a, 0x02, 1, 2]);

describe('non-unique identity complete proof wire format', () => {
  it('should round trip a standard Proof at field 4 across all published JS clients', () => {
    const response = new nodeMessages[name]();
    const v0 = new nodeMessages[name][innerName]();
    const proof = new nodeMessages.Proof();
    proof.setGrovedbProof(Buffer.from([1, 2]));
    v0.setProof(proof);
    response.setV0(v0);
    assert.deepEqual(Buffer.from(response.serializeBinary()), singleWire);
    const web = webMessages[name].deserializeBinary(singleWire);
    assert.deepEqual(Buffer.from(web.getV0().getProof().getGrovedbProof_asU8()), Buffer.from([1, 2]));
    [protobufMessages, driveMessages].forEach((messages) => {
      const Response = messages.org.dash.platform.dapi.v0[name];
      const decoded = Response.decode(singleWire);
      assert.deepEqual(Buffer.from(decoded.v0.proof.grovedbProof), Buffer.from([1, 2]));
      assert.deepEqual(Buffer.from(Response.encode(decoded).finish()), singleWire);
    });
  });

  it('should discard retired field 2 without treating it as a supported proof', () => {
    [nodeMessages, webMessages].forEach((messages) => {
      assert.equal(messages[name].deserializeBinary(retiredWire).getV0().hasProof(), false);
      assert.equal(messages[name][innerName].IdentityProvedResponse, undefined);
    });
    [protobufMessages, driveMessages].forEach((messages) => {
      const Response = messages.org.dash.platform.dapi.v0[name];
      assert.equal(Response.decode(retiredWire).v0.proof, null);
      assert.equal(Response[innerName].IdentityProvedResponse, undefined);
    });
  });
});
