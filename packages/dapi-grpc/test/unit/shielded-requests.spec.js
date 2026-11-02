const assert = require('node:assert/strict');
const nodeMessages = require('../../clients/platform/v0/nodejs/platform_protoc');
const webMessages = require('../../clients/platform/v0/web/platform_pb');
const protobufMessages = require('../../clients/platform/v0/nodejs/platform_pbjs');
const driveMessages = require('../../clients/drive/v0/nodejs/drive_pbjs');

const requestNames = [
  'GetShieldedEncryptedNotesRequest',
  'GetShieldedAnchorsRequest',
  'GetMostRecentShieldedAnchorRequest',
  'GetShieldedPoolStateRequest',
  'GetShieldedNotesCountRequest',
  'GetShieldedNullifiersRequest',
];

describe('shielded query token selectors', () => {
  requestNames.forEach((name) => {
    describe(name, () => {
      const NodeRequest = nodeMessages[name][`${name}V0`];
      const WebRequest = webMessages[name][`${name}V0`];
      const ProtobufRequest = protobufMessages.org.dash.platform.dapi.v0[name][`${name}V0`];
      const DriveRequest = driveMessages.org.dash.platform.dapi.v0[name][`${name}V0`];

      [undefined, Buffer.alloc(32, 7)].forEach((tokenId) => {
        it(`should preserve the ${tokenId ? 'token' : 'credit'} pool selector across clients`, () => {
          const request = new NodeRequest();
          if (tokenId) {
            request.setTokenId(tokenId);
          }
          const webRequest = WebRequest.deserializeBinary(request.serializeBinary());
          assert.equal(webRequest.hasTokenId(), Boolean(tokenId));
          if (tokenId) {
            assert.deepEqual(Buffer.from(webRequest.getTokenId_asU8()), tokenId);
          }

          const protobufRequest = ProtobufRequest.decode(webRequest.serializeBinary());
          assert.equal(Object.hasOwn(protobufRequest, 'tokenId'), Boolean(tokenId));
          const driveRequest = DriveRequest.decode(ProtobufRequest.encode(protobufRequest).finish());
          assert.equal(Object.hasOwn(driveRequest, 'tokenId'), Boolean(tokenId));
          const roundTrip = NodeRequest.deserializeBinary(DriveRequest.encode(driveRequest).finish());
          assert.equal(roundTrip.hasTokenId(), Boolean(tokenId));
          if (tokenId) {
            assert.deepEqual(Buffer.from(roundTrip.getTokenId_asU8()), tokenId);
            roundTrip.clearTokenId();
            assert.equal(roundTrip.hasTokenId(), false);
          }
        });
      });
    });
  });
});
