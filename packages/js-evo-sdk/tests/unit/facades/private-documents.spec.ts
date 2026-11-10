import type { SinonStub } from 'sinon';
import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('PrivateDocumentsFacade', () => {
  let wasmSdk: wasmSDKPackage.WasmSdk;
  let client: EvoSDK;

  let getPrivateDocumentEntriesStub: SinonStub;
  let sealPrivateContactRequestStub: SinonStub;
  let scanPrivateContactRequestsStub: SinonStub;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnet();
    wasmSdk = await builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    getPrivateDocumentEntriesStub = this.sinon
      .stub(wasmSdk, 'getPrivateDocumentEntries')
      .resolves({ entries: [], totalCount: 0n });
    sealPrivateContactRequestStub = this.sinon
      .stub(wasmSdk, 'sealPrivateContactRequest')
      .resolves(new Uint8Array(589));
    scanPrivateContactRequestsStub = this.sinon
      .stub(wasmSdk, 'scanPrivateContactRequests')
      .resolves('scan');
  });

  describe('entries()', () => {
    it('should forward the contract, type and start position to getPrivateDocumentEntries', async () => {
      const big = 9_007_199_254_740_993n; // > Number.MAX_SAFE_INTEGER
      const result = await client.privateDocuments.entries('contract', 'privateContactRequest', big);
      expect(getPrivateDocumentEntriesStub).to.be.calledOnceWithExactly(
        'contract',
        'privateContactRequest',
        big,
      );
      expect(result).to.deep.equal({ entries: [], totalCount: 0n });
    });
  });

  describe('sealContactRequest()', () => {
    it('should forward the options to sealPrivateContactRequest and return the entry', async () => {
      const options = { recipientKeyId: 2 } as unknown as wasmSDKPackage.SealPrivateContactRequestOptions;
      const entry = await client.privateDocuments.sealContactRequest(options);
      expect(sealPrivateContactRequestStub).to.be.calledOnceWithExactly(options);
      expect(entry).to.have.length(589);
    });
  });

  describe('scanContactRequests()', () => {
    it('should forward the options to scanPrivateContactRequests', async () => {
      const options = { startPosition: 0n };
      const result = await client.privateDocuments.scanContactRequests(options);
      expect(scanPrivateContactRequestsStub).to.be.calledOnceWithExactly(options);
      expect(result).to.equal('scan');
    });
  });
});
