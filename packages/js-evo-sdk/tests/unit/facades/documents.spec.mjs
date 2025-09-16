import { EvoSDK } from '../../../dist/sdk.js';
import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';

describe('DocumentsFacade', () => {
  let wasmSdk;
  let client;

  beforeEach(async function () {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    this.sinon.stub(wasmSdk, 'getDocuments').resolves('ok');
    this.sinon.stub(wasmSdk, 'getDocumentsWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getDocument').resolves('ok');
    this.sinon.stub(wasmSdk, 'getDocumentWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'documentCreate').resolves('ok');
    this.sinon.stub(wasmSdk, 'documentReplace').resolves('ok');
    this.sinon.stub(wasmSdk, 'documentDelete').resolves('ok');
    this.sinon.stub(wasmSdk, 'documentTransfer').resolves('ok');
    this.sinon.stub(wasmSdk, 'documentPurchase').resolves('ok');
    this.sinon.stub(wasmSdk, 'documentSetPrice').resolves('ok');
  });

  it('query() forwards to wasm.getDocuments with JSON and null handling', async () => {
    const where = { a: 1 };
    const order = { b: 'asc' };
    await client.documents.query({ contractId: 'c', type: 't', where, orderBy: order, limit: 5, startAfter: 'x', startAt: 'y' });
    expect(wasmSdk.getDocuments).to.be.calledOnceWithExactly('c', 't', JSON.stringify(where), JSON.stringify(order), 5, 'x', 'y');
  });

  it('queryWithProof() forwards to wasm.getDocumentsWithProofInfo', async () => {
    await client.documents.queryWithProof({ contractId: 'c', type: 't' });
    expect(wasmSdk.getDocumentsWithProofInfo).to.be.calledOnceWithExactly('c', 't', null, null, null, null, null);
  });

  it('get() forwards to wasm.getDocument', async () => {
    await client.documents.get('c', 't', 'id');
    expect(wasmSdk.getDocument).to.be.calledOnceWithExactly('c', 't', 'id');
  });

  it('getWithProof() forwards to wasm.getDocumentWithProofInfo', async () => {
    await client.documents.getWithProof('c', 't', 'id');
    expect(wasmSdk.getDocumentWithProofInfo).to.be.calledOnceWithExactly('c', 't', 'id');
  });

  it('create() calls wasmSdk.documentCreate with JSON data', async () => {
    const data = { foo: 'bar' };
    await client.documents.create({ contractId: 'c', type: 't', ownerId: 'o', data, entropyHex: 'ee', privateKeyWif: 'wif' });
    expect(wasmSdk.documentCreate).to.be.calledOnceWithExactly('c', 't', 'o', JSON.stringify(data), 'ee', 'wif');
  });

  it('replace() calls wasmSdk.documentReplace with BigInt revision', async () => {
    await client.documents.replace({ contractId: 'c', type: 't', documentId: 'id', ownerId: 'o', data: { n: 1 }, revision: 2, privateKeyWif: 'w' });
    expect(wasmSdk.documentReplace).to.be.calledOnce();
    const [c, t, id, o, json, rev, w] = wasmSdk.documentReplace.firstCall.args;
    expect([c, t, id, o, w]).to.deep.equal(['c', 't', 'id', 'o', 'w']);
    expect(json).to.equal(JSON.stringify({ n: 1 }));
    expect(typeof rev).to.equal('bigint');
    expect(rev).to.equal(2n);
  });

  it('delete() calls wasmSdk.documentDelete', async () => {
    await client.documents.delete({ contractId: 'c', type: 't', documentId: 'id', ownerId: 'o', privateKeyWif: 'w' });
    expect(wasmSdk.documentDelete).to.be.calledOnceWithExactly('c', 't', 'id', 'o', 'w');
  });

  it('transfer() calls wasmSdk.documentTransfer', async () => {
    await client.documents.transfer({ contractId: 'c', type: 't', documentId: 'id', ownerId: 'o', recipientId: 'r', privateKeyWif: 'w' });
    expect(wasmSdk.documentTransfer).to.be.calledOnceWithExactly('c', 't', 'id', 'o', 'r', 'w');
  });

  it('purchase() calls wasmSdk.documentPurchase with BigInt amount', async () => {
    await client.documents.purchase({ contractId: 'c', type: 't', documentId: 'id', buyerId: 'b', price: '7', privateKeyWif: 'w' });
    const args = wasmSdk.documentPurchase.firstCall.args;
    expect(args[0]).to.equal('c');
    expect(args[1]).to.equal('t');
    expect(args[2]).to.equal('id');
    expect(args[3]).to.equal('b');
    expect(typeof args[4]).to.equal('bigint');
    expect(args[4]).to.equal(7n);
    expect(args[5]).to.equal('w');
  });

  it('setPrice() calls wasmSdk.documentSetPrice with BigInt price', async () => {
    await client.documents.setPrice({ contractId: 'c', type: 't', documentId: 'id', ownerId: 'o', price: 9, privateKeyWif: 'w' });
    const args = wasmSdk.documentSetPrice.firstCall.args;
    expect(args[0]).to.equal('c');
    expect(args[1]).to.equal('t');
    expect(args[2]).to.equal('id');
    expect(args[3]).to.equal('o');
    expect(typeof args[4]).to.equal('bigint');
    expect(args[4]).to.equal(9n);
    expect(args[5]).to.equal('w');
  });
});
