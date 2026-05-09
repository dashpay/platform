import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

/**
 * Unit tests for synchronous document transition input validation.
 *
 * These tests only cover the validation branches that fire **synchronously**
 * before any network access (revision guards, entropy guards) for the prepare
 * and one-shot APIs. The happy-path tests that need a fetched data contract
 * live in `tests/functional/transitions/documents.spec.ts`.
 *
 * The WasmSdk is constructed via `WasmSdkBuilder.testnet().build()`, which
 * does not require a live platform — we expect every call here to reject
 * before any `get_or_fetch_contract` call runs.
 */

const DUMMY_ID = '11111111111111111111111111111111';
const DUMMY_ID_2 = '22222222222222222222222222222222';

function buildSigner() {
  // createTestSignerAndKey needs a running platform for functional test data,
  // but constructing a signer + key by hand only exercises static APIs.
  const signer = new sdk.IdentitySigner();
  const privateKey = sdk.PrivateKey.fromHex(
    '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
    'testnet',
  );
  signer.addKey(privateKey);

  const identityKey = new sdk.IdentityPublicKey({
    keyId: 0,
    purpose: 'AUTHENTICATION',
    securityLevel: 'HIGH',
    keyType: 'ECDSA_SECP256K1',
    isReadOnly: false,
    data: privateKey.getPublicKey().toBytes(),
  });

  return { signer, identityKey };
}

function buildDocument(overrides: Record<string, unknown> = {}) {
  return new sdk.Document({
    properties: { message: 'hello' },
    documentTypeName: 'note',
    dataContractId: DUMMY_ID,
    ownerId: DUMMY_ID_2,
    ...overrides,
  });
}

describe('prepareDocument* validation', function describePrepareDocumentValidation() {
  this.timeout(30000);

  let client: sdk.WasmSdk;

  before(async () => {
    await init();
    const builder = sdk.WasmSdkBuilder.testnet();
    client = await builder.build();
  });

  after(() => {
    if (client) { client.free(); }
  });

  describe('prepareDocumentCreate()', () => {
    it('rejects a document with no entropy', async () => {
      const { signer, identityKey } = buildSigner();
      const document = buildDocument();
      // Entropy defaults to a generated 32-byte value in the constructor; clear it
      // to hit the "must have entropy set" guard in prepare_document_create.
      document.entropy = null;

      try {
        await client.prepareDocumentCreate({ document, identityKey, signer });
        expect.fail('expected prepareDocumentCreate to reject without entropy');
      } catch (e) {
        expect(e).to.be.instanceOf(sdk.WasmSdkError);
        expect(e.name).to.equal('InvalidArgument');
        expect(e.message).to.match(/entropy/i);
      }
    });

    it('rejects a document whose entropy is not 32 bytes', async () => {
      // NOTE: the Document constructor and the entropy setter both reject
      // non-32-byte buffers, so the defensive length check inside
      // prepare_document_create is unreachable from JS under normal use. We
      // assert the outer guard here — that the SDK refuses bad entropy
      // *somewhere* before broadcasting — which is the behavior callers care
      // about.
      try {
        const document = buildDocument();
        document.entropy = new Uint8Array(16);
        expect.fail('expected Document entropy setter to reject a 16-byte buffer');
      } catch (e) {
        expect(e).to.be.instanceOf(sdk.WasmSdkError);
        expect(e.name).to.equal('InvalidArgument');
        expect(e.message).to.match(/entropy/i);
      }
    });

    it('rejects a document with revision > INITIAL_REVISION (would silently be a replace)', async () => {
      const { signer, identityKey } = buildSigner();
      const document = buildDocument({ revision: 2 });

      try {
        await client.prepareDocumentCreate({ document, identityKey, signer });
        expect.fail('expected prepareDocumentCreate to reject revision > 1');
      } catch (e) {
        expect(e).to.be.instanceOf(sdk.WasmSdkError);
        expect(e.name).to.equal('InvalidArgument');
        expect(e.message).to.match(/revision/i);
        expect(e.message).to.match(/prepareDocumentReplace/);
      }
    });

    it('rejects a document with revision 0 (would silently be a replace)', async () => {
      // `build_document_create_or_replace_transition` routes any `revision != INITIAL_REVISION`
      // to the replace branch, so `revision = 0` must be rejected too — otherwise it would
      // silently produce a replace transition instead of a create.
      const { signer, identityKey } = buildSigner();
      const document = buildDocument({ revision: 0 });

      try {
        await client.prepareDocumentCreate({ document, identityKey, signer });
        expect.fail('expected prepareDocumentCreate to reject revision 0');
      } catch (e) {
        expect(e).to.be.instanceOf(sdk.WasmSdkError);
        expect(e.name).to.equal('InvalidArgument');
        expect(e.message).to.match(/revision/i);
        expect(e.message).to.match(/prepareDocumentReplace/);
      }
    });
  });

  describe('prepareDocumentReplace()', () => {
    it('rejects a document with no revision', async () => {
      const { signer, identityKey } = buildSigner();
      const document = buildDocument();
      // The Document constructor defaults revision to 1; clear it to exercise
      // the "must have a revision set" guard in prepare_document_replace.
      document.revision = null;

      try {
        await client.prepareDocumentReplace({ document, identityKey, signer });
        expect.fail('expected prepareDocumentReplace to reject missing revision');
      } catch (e) {
        expect(e).to.be.instanceOf(sdk.WasmSdkError);
        expect(e.name).to.equal('InvalidArgument');
        expect(e.message).to.match(/revision/i);
      }
    });

    it('rejects a document with revision 0', async () => {
      const { signer, identityKey } = buildSigner();
      const document = buildDocument({ revision: 0 });

      try {
        await client.prepareDocumentReplace({ document, identityKey, signer });
        expect.fail('expected prepareDocumentReplace to reject revision 0');
      } catch (e) {
        expect(e).to.be.instanceOf(sdk.WasmSdkError);
        expect(e.name).to.equal('InvalidArgument');
        expect(e.message).to.match(/revision/i);
        expect(e.message).to.match(/prepareDocumentCreate/);
      }
    });

    it('rejects a document with revision 1 (INITIAL_REVISION)', async () => {
      const { signer, identityKey } = buildSigner();
      const document = buildDocument({ revision: 1 });

      try {
        await client.prepareDocumentReplace({ document, identityKey, signer });
        expect.fail('expected prepareDocumentReplace to reject revision 1');
      } catch (e) {
        expect(e).to.be.instanceOf(sdk.WasmSdkError);
        expect(e.name).to.equal('InvalidArgument');
        expect(e.message).to.match(/revision/i);
        expect(e.message).to.match(/prepareDocumentCreate/);
      }
    });
  });

  describe('prepareDocumentDelete()', () => {
    it('rejects when document is missing', async () => {
      const { signer, identityKey } = buildSigner();

      try {
        await client.prepareDocumentDelete({ identityKey, signer });
        expect.fail('expected prepareDocumentDelete to reject without document');
      } catch (e) {
        expect(e).to.be.instanceOf(sdk.WasmSdkError);
        expect(e.name).to.equal('InvalidArgument');
        expect(e.message).to.match(/document is required/i);
      }
    });

    it('rejects when document is null', async () => {
      const { signer, identityKey } = buildSigner();

      try {
        await client.prepareDocumentDelete({ document: null, identityKey, signer });
        expect.fail('expected prepareDocumentDelete to reject null document');
      } catch (e) {
        expect(e).to.be.instanceOf(sdk.WasmSdkError);
        expect(e.name).to.equal('InvalidArgument');
        expect(e.message).to.match(/document is required/i);
      }
    });

    it('rejects a plain object with no id', async () => {
      const { signer, identityKey } = buildSigner();

      try {
        await client.prepareDocumentDelete({
          document: {
            ownerId: DUMMY_ID_2,
            dataContractId: DUMMY_ID,
            documentTypeName: 'note',
          },
          identityKey,
          signer,
        });
        expect.fail('expected prepareDocumentDelete to reject missing id');
      } catch (e) {
        expect(e).to.be.instanceOf(sdk.WasmSdkError);
        expect(e.name).to.equal('InvalidArgument');
        expect(e.message).to.match(/id/i);
      }
    });

    it('rejects a plain object with no ownerId', async () => {
      const { signer, identityKey } = buildSigner();

      try {
        await client.prepareDocumentDelete({
          document: {
            id: DUMMY_ID,
            dataContractId: DUMMY_ID,
            documentTypeName: 'note',
          },
          identityKey,
          signer,
        });
        expect.fail('expected prepareDocumentDelete to reject missing ownerId');
      } catch (e) {
        expect(e).to.be.instanceOf(sdk.WasmSdkError);
        expect(e.name).to.equal('InvalidArgument');
        expect(e.message).to.match(/ownerId/i);
      }
    });

    it('rejects a plain object with no dataContractId', async () => {
      const { signer, identityKey } = buildSigner();

      try {
        await client.prepareDocumentDelete({
          document: {
            id: DUMMY_ID,
            ownerId: DUMMY_ID_2,
            documentTypeName: 'note',
          },
          identityKey,
          signer,
        });
        expect.fail('expected prepareDocumentDelete to reject missing dataContractId');
      } catch (e) {
        expect(e).to.be.instanceOf(sdk.WasmSdkError);
        expect(e.name).to.equal('InvalidArgument');
        expect(e.message).to.match(/dataContractId/i);
      }
    });

    it('rejects a plain object with no documentTypeName', async () => {
      const { signer, identityKey } = buildSigner();

      try {
        await client.prepareDocumentDelete({
          document: {
            id: DUMMY_ID,
            ownerId: DUMMY_ID_2,
            dataContractId: DUMMY_ID,
          },
          identityKey,
          signer,
        });
        expect.fail('expected prepareDocumentDelete to reject missing documentTypeName');
      } catch (e) {
        expect(e).to.be.instanceOf(sdk.WasmSdkError);
        expect(e.name).to.equal('InvalidArgument');
        expect(e.message).to.match(/documentTypeName/i);
      }
    });

    it('rejects a Document instance when identityKey has the wrong shape', async () => {
      const { signer } = buildSigner();
      const document = buildDocument({ revision: 1 });

      try {
        await client.prepareDocumentDelete({
          document,
          identityKey: {},
          signer,
        });
        expect.fail('expected prepareDocumentDelete to reject invalid identityKey');
      } catch (e) {
        expect(e).to.be.instanceOf(sdk.WasmSdkError);
        expect(e.name).to.equal('InvalidArgument');
        expect(e.message).to.match(/identityKey/i);
      }
    });
  });

  describe('documentCreate()/documentReplace()', () => {
    it('documentCreate rejects a document with revision 0', async () => {
      const { signer, identityKey } = buildSigner();
      const document = buildDocument({ revision: 0 });

      try {
        await client.documentCreate({ document, identityKey, signer });
        expect.fail('expected documentCreate to reject revision 0');
      } catch (e) {
        expect(e).to.be.instanceOf(sdk.WasmSdkError);
        expect(e.name).to.equal('InvalidArgument');
        expect(e.message).to.match(/revision/i);
        expect(e.message).to.match(/documentReplace/);
      }
    });

    it('documentReplace rejects a document with revision 1 (INITIAL_REVISION)', async () => {
      const { signer, identityKey } = buildSigner();
      const document = buildDocument({ revision: 1 });

      try {
        await client.documentReplace({ document, identityKey, signer });
        expect.fail('expected documentReplace to reject revision 1');
      } catch (e) {
        expect(e).to.be.instanceOf(sdk.WasmSdkError);
        expect(e.name).to.equal('InvalidArgument');
        expect(e.message).to.match(/revision/i);
        expect(e.message).to.match(/documentCreate/);
      }
    });
  });
});
