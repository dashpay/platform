import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('DocumentsFacade', () => {
  let wasmSdk;
  let client;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    this.sinon.stub(wasmSdk, 'getDocuments').resolves('ok');
    this.sinon.stub(wasmSdk, 'getDocumentsWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getDocument').resolves('ok');
    this.sinon.stub(wasmSdk, 'getDocumentWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'documentCreate').resolves();
    this.sinon.stub(wasmSdk, 'documentReplace').resolves();
    this.sinon.stub(wasmSdk, 'documentDelete').resolves();
    this.sinon.stub(wasmSdk, 'documentTransfer').resolves();
    this.sinon.stub(wasmSdk, 'documentPurchase').resolves();
    this.sinon.stub(wasmSdk, 'documentSetPrice').resolves();
  });

  it('query() forwards DocumentsQuery', async () => {
    const query = {
      dataContractId: 'c',
      documentTypeName: 't',
      where: [['field', '==', 'value']],
      orderBy: [['field', 'asc']],
      limit: 5,
      startAfter: 'x',
    };
    await client.documents.query(query);
    expect(wasmSdk.getDocuments).to.be.calledOnceWithExactly(query);
  });

  it('queryWithProof() forwards DocumentsQuery', async () => {
    const query = {
      dataContractId: 'c',
      documentTypeName: 't',
    };
    await client.documents.queryWithProof(query);
    expect(wasmSdk.getDocumentsWithProofInfo).to.be.calledOnceWithExactly(query);
  });

  it('get() forwards to wasm.getDocument', async () => {
    await client.documents.get('c', 't', 'id');
    expect(wasmSdk.getDocument).to.be.calledOnceWithExactly('c', 't', 'id');
  });

  it('getWithProof() forwards to wasm.getDocumentWithProofInfo', async () => {
    await client.documents.getWithProof('c', 't', 'id');
    expect(wasmSdk.getDocumentWithProofInfo).to.be.calledOnceWithExactly('c', 't', 'id');
  });

  it('create() calls wasmSdk.documentCreate with options', async () => {
    const options = {
      contractId: 'c',
      type: 't',
      ownerId: 'o',
      data: { foo: 'bar' },
      signer: { privateKeyWif: 'wif' },
    };
    await client.documents.create(options);
    expect(wasmSdk.documentCreate).to.be.calledOnceWithExactly(options);
  });

  it('replace() calls wasmSdk.documentReplace with options', async () => {
    const options = {
      contractId: 'c',
      type: 't',
      documentId: 'id',
      ownerId: 'o',
      data: { n: 1 },
      signer: { privateKeyWif: 'w' },
    };
    await client.documents.replace(options);
    expect(wasmSdk.documentReplace).to.be.calledOnceWithExactly(options);
  });

  it('delete() calls wasmSdk.documentDelete with options', async () => {
    const options = {
      contractId: 'c',
      type: 't',
      documentId: 'id',
      ownerId: 'o',
      signer: { privateKeyWif: 'w' },
    };
    await client.documents.delete(options);
    expect(wasmSdk.documentDelete).to.be.calledOnceWithExactly(options);
  });

  it('transfer() calls wasmSdk.documentTransfer with options', async () => {
    const options = {
      contractId: 'c',
      type: 't',
      documentId: 'id',
      ownerId: 'o',
      recipientId: 'r',
      signer: { privateKeyWif: 'w' },
    };
    await client.documents.transfer(options);
    expect(wasmSdk.documentTransfer).to.be.calledOnceWithExactly(options);
  });

  it('purchase() calls wasmSdk.documentPurchase with options', async () => {
    const options = {
      contractId: 'c',
      type: 't',
      documentId: 'id',
      buyerId: 'b',
      price: BigInt(7),
      signer: { privateKeyWif: 'w' },
    };
    await client.documents.purchase(options);
    expect(wasmSdk.documentPurchase).to.be.calledOnceWithExactly(options);
  });

  it('setPrice() calls wasmSdk.documentSetPrice with options', async () => {
    const options = {
      contractId: 'c',
      type: 't',
      documentId: 'id',
      ownerId: 'o',
      price: BigInt(9),
      signer: { privateKeyWif: 'w' },
    };
    await client.documents.setPrice(options);
    expect(wasmSdk.documentSetPrice).to.be.calledOnceWithExactly(options);
  });
});
