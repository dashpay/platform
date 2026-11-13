const assert = require('node:assert/strict');
const nodeMessages = require('../../clients/platform/v0/nodejs/platform_protoc');
const webMessages = require('../../clients/platform/v0/web/platform_pb');
const protobufMessages = require('../../clients/platform/v0/nodejs/platform_pbjs');
const driveMessages = require('../../clients/drive/v0/nodejs/drive_pbjs');

describe('document history versioned wire format', () => {
  const requestWire = Buffer.from([0x0a, 6, 0x2a, 2, 8, 1, 0x38, 1]);
  const responseWire = Buffer.from([0x0a, 10, 0x0a, 8, 0x0a, 6, 8, 1, 0x12, 2, 255, 255]);

  it('should preserve the offset wrapper and proof flag of a version 0 request', () => {
    [nodeMessages, webMessages].forEach((messages) => {
      const request = messages.GetDocumentHistoryRequest.deserializeBinary(requestWire);
      assert.equal(request.getV0().getOffset().getValue(), 1);
      assert.equal(request.getV0().getProve(), true);
      assert.deepEqual(Buffer.from(request.serializeBinary()), requestWire);
    });
    [protobufMessages, driveMessages].forEach((messages) => {
      const Request = messages.org.dash.platform.dapi.v0.GetDocumentHistoryRequest;
      const request = Request.decode(requestWire);
      assert.equal(request.v0.offset.value, 1);
      assert.equal(request.v0.prove, true);
      assert.deepEqual(Buffer.from(Request.encode(request).finish()), requestWire);
    });
  });

  it('should preserve document bytes at field 2 of a version 0 history entry', () => {
    [nodeMessages, webMessages].forEach((messages) => {
      const response = messages.GetDocumentHistoryResponse.deserializeBinary(responseWire);
      const entry = response.getV0().getDocumentHistory().getDocumentEntriesList()[0];
      assert.equal(entry.getDate(), '1');
      assert.deepEqual(Buffer.from(entry.getValue_asU8()), Buffer.from([255, 255]));
      assert.deepEqual(Buffer.from(response.serializeBinary()), responseWire);
    });
    [protobufMessages, driveMessages].forEach((messages) => {
      const Response = messages.org.dash.platform.dapi.v0.GetDocumentHistoryResponse;
      const response = Response.decode(responseWire);
      const entry = response.v0.documentHistory.documentEntries[0];
      assert.equal(entry.date.toString(), '1');
      assert.deepEqual(Buffer.from(entry.value), Buffer.from([255, 255]));
      assert.deepEqual(Buffer.from(Response.encode(response).finish()), responseWire);
    });
  });
  it('should carry composite selectors in version 1 at outer field 2', () => {
    const bytes = Buffer.from([0x12, 4, 0x28, 1, 0x40, 2]);
    [nodeMessages, webMessages].forEach((messages) => {
      const request = messages.GetDocumentHistoryRequest.deserializeBinary(bytes);
      assert.equal(request.getV1().getProve(), true);
      assert.equal(request.getV1().getStartAtRevision(), '2');
      assert.equal(request.hasV0(), false);
      assert.deepEqual(Buffer.from(request.serializeBinary()), bytes);
    });
    [protobufMessages, driveMessages].forEach((messages) => {
      const Request = messages.org.dash.platform.dapi.v0.GetDocumentHistoryRequest;
      const request = Request.decode(bytes);
      assert.equal(request.v1.prove, true);
      assert.equal(request.v1.startAtRevision.toString(), '2');
      assert.equal(request.v0, null);
      assert.deepEqual(Buffer.from(Request.encode(request).finish()), bytes);
    });
  });

  it('should preserve deleted and erasing lifecycle metadata across published clients', () => {
    [2, 3].forEach((state) => {
      const bytes = Buffer.from([
        0x12, 16, 0x0a, 14, 0x12, 12,
        8, state, 16, 1, 24, 5, 32, 6, 40, 4, 48, 7,
      ]);
      [nodeMessages, webMessages].forEach((messages) => {
        const response = messages.GetDocumentHistoryResponse.deserializeBinary(bytes);
        const lifecycle = response.getV1().getHistory().getLifecycle();
        assert.equal(lifecycle.getState(), state);
        assert.equal(lifecycle.getRemainingRevisions(), '1');
        assert.equal(lifecycle.getDeletedAtMs(), '5');
        assert.equal(lifecycle.getErasingStartedAtMs(), '6');
        assert.equal(lifecycle.getErasingFromTimeMs(), '4');
        assert.equal(lifecycle.getErasingFromRevision(), '7');
        assert.deepEqual(Buffer.from(response.serializeBinary()), bytes);
      });
      [protobufMessages, driveMessages].forEach((messages) => {
        const Response = messages.org.dash.platform.dapi.v0.GetDocumentHistoryResponse;
        const response = Response.decode(bytes);
        const { lifecycle } = response.v1.history;
        assert.equal(lifecycle.state, state);
        assert.equal(lifecycle.remainingRevisions.toString(), '1');
        assert.equal(lifecycle.deletedAtMs.toString(), '5');
        assert.equal(lifecycle.erasingStartedAtMs.toString(), '6');
        assert.equal(lifecycle.erasingFromTimeMs.toString(), '4');
        assert.equal(lifecycle.erasingFromRevision.toString(), '7');
        assert.deepEqual(Buffer.from(Response.encode(response).finish()), bytes);
      });
    });
  });
});
