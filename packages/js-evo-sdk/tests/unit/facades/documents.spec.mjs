import { EvoSDK } from '../../../dist/evo-sdk.module.js';
import sinon from 'sinon';

const isBrowser = typeof window !== 'undefined';

describe('DocumentsFacade', () => {
  if (!isBrowser) {
    it('skips in Node environment (browser-only)', function () { this.skip(); });
    return;
  }

  let wasmStubModule;
  before(async () => {
    // Karma aliases '@dashevo/wasm-sdk' to a local stub with __getCalls helpers
    wasmStubModule = await import('@dashevo/wasm-sdk');
  });

  beforeEach(() => {
    wasmStubModule.__clearCalls();
  });

  it('query() forwards to wasm.get_documents with JSON and null handling', async () => {
    const raw = {}; // WasmSdk instance is passed to free functions
    const sdk = EvoSDK.fromWasm(raw);
    const where = { a: 1 };
    const order = { b: 'asc' };
    await sdk.documents.query({ contractId: 'c', type: 't', where, orderBy: order, limit: 5, startAfter: 'x', startAt: 'y' });

    const calls = wasmStubModule.__getCalls();
    const last = calls[calls.length - 1];
    expect(last.called).to.equal('get_documents');
    expect(last.args[0]).to.equal(raw);
    expect(last.args.slice(1)).to.deep.equal(['c', 't', JSON.stringify(where), JSON.stringify(order), 5, 'x', 'y']);
  });

  it('queryWithProof() forwards to wasm.get_documents_with_proof_info', async () => {
    const raw = {};
    const sdk = EvoSDK.fromWasm(raw);
    await sdk.documents.queryWithProof({ contractId: 'c', type: 't' });
    const last = wasmStubModule.__getCalls().pop();
    expect(last.called).to.equal('get_documents_with_proof_info');
    expect(last.args.slice(0, 3)).to.deep.equal([raw, 'c', 't']);
  });

  it('get() forwards to wasm.get_document', async () => {
    const raw = {};
    const sdk = EvoSDK.fromWasm(raw);
    await sdk.documents.get('c', 't', 'id');
    const last = wasmStubModule.__getCalls().pop();
    expect(last.called).to.equal('get_document');
    expect(last.args).to.deep.equal([raw, 'c', 't', 'id']);
  });

  it('getWithProof() forwards to wasm.get_document_with_proof_info', async () => {
    const raw = {};
    const sdk = EvoSDK.fromWasm(raw);
    await sdk.documents.getWithProof('c', 't', 'id');
    const last = wasmStubModule.__getCalls().pop();
    expect(last.called).to.equal('get_document_with_proof_info');
    expect(last.args).to.deep.equal([raw, 'c', 't', 'id']);
  });

  it('create() calls wasmSdk.documentCreate with JSON data', async () => {
    const wasm = { documentCreate: sinon.stub().resolves('ok') };
    const sdk = EvoSDK.fromWasm(wasm);
    const data = { foo: 'bar' };
    await sdk.documents.create({ contractId: 'c', type: 't', ownerId: 'o', data, entropyHex: 'ee', privateKeyWif: 'wif' });
    sinon.assert.calledOnceWithExactly(wasm.documentCreate, 'c', 't', 'o', JSON.stringify(data), 'ee', 'wif');
  });

  it('replace() calls wasmSdk.documentReplace with BigInt revision', async () => {
    const wasm = { documentReplace: sinon.stub().resolves('ok') };
    const sdk = EvoSDK.fromWasm(wasm);
    await sdk.documents.replace({ contractId: 'c', type: 't', documentId: 'id', ownerId: 'o', data: { n: 1 }, revision: 2, privateKeyWif: 'w' });
    sinon.assert.calledOnce(wasm.documentReplace);
    const [c, t, id, o, json, rev, w] = wasm.documentReplace.firstCall.args;
    expect([c, t, id, o, w]).to.deep.equal(['c', 't', 'id', 'o', 'w']);
    expect(json).to.equal(JSON.stringify({ n: 1 }));
    expect(typeof rev).to.equal('bigint');
    expect(rev).to.equal(2n);
  });

  it('delete() calls wasmSdk.documentDelete', async () => {
    const wasm = { documentDelete: sinon.stub().resolves('ok') };
    const sdk = EvoSDK.fromWasm(wasm);
    await sdk.documents.delete({ contractId: 'c', type: 't', documentId: 'id', ownerId: 'o', privateKeyWif: 'w' });
    sinon.assert.calledOnceWithExactly(wasm.documentDelete, 'c', 't', 'id', 'o', 'w');
  });

  it('transfer() calls wasmSdk.documentTransfer', async () => {
    const wasm = { documentTransfer: sinon.stub().resolves('ok') };
    const sdk = EvoSDK.fromWasm(wasm);
    await sdk.documents.transfer({ contractId: 'c', type: 't', documentId: 'id', ownerId: 'o', recipientId: 'r', privateKeyWif: 'w' });
    sinon.assert.calledOnceWithExactly(wasm.documentTransfer, 'c', 't', 'id', 'o', 'r', 'w');
  });

  it('purchase() calls wasmSdk.documentPurchase with BigInt amount', async () => {
    const wasm = { documentPurchase: sinon.stub().resolves('ok') };
    const sdk = EvoSDK.fromWasm(wasm);
    await sdk.documents.purchase({ contractId: 'c', type: 't', documentId: 'id', buyerId: 'b', price: '7', privateKeyWif: 'w' });
    const args = wasm.documentPurchase.firstCall.args;
    expect(args[0]).to.equal('c');
    expect(args[1]).to.equal('t');
    expect(args[2]).to.equal('id');
    expect(args[3]).to.equal('b');
    expect(typeof args[4]).to.equal('bigint');
    expect(args[4]).to.equal(7n);
    expect(args[5]).to.equal('w');
  });

  it('setPrice() calls wasmSdk.documentSetPrice with BigInt price', async () => {
    const wasm = { documentSetPrice: sinon.stub().resolves('ok') };
    const sdk = EvoSDK.fromWasm(wasm);
    await sdk.documents.setPrice({ contractId: 'c', type: 't', documentId: 'id', ownerId: 'o', price: 9, privateKeyWif: 'w' });
    const args = wasm.documentSetPrice.firstCall.args;
    expect(args[0]).to.equal('c');
    expect(args[1]).to.equal('t');
    expect(args[2]).to.equal('id');
    expect(args[3]).to.equal('o');
    expect(typeof args[4]).to.equal('bigint');
    expect(args[4]).to.equal(9n);
    expect(args[5]).to.equal('w');
  });
});

