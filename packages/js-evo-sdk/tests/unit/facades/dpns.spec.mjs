import { EvoSDK } from '../../../dist/evo-sdk.module.js';

const isBrowser = typeof window !== 'undefined';

describe('DpnsFacade', () => {
  if (!isBrowser) {
    it('skips in Node environment (browser-only)', function () { this.skip(); });
    return;
  }

  let wasmStubModule;
  before(async () => { wasmStubModule = await import('@dashevo/wasm-sdk'); });
  beforeEach(() => { wasmStubModule.__clearCalls(); });

  it('convertToHomographSafe/isValidUsername/isContestedUsername call pure functions', () => {
    // These are synchronous functions
    const out1 = wasmStubModule.dpns_convert_to_homograph_safe('abc');
    const out2 = wasmStubModule.dpns_is_valid_username('abc');
    const out3 = wasmStubModule.dpns_is_contested_username('abc');
    expect(out1).to.be.ok(); // stub returns record object
    expect(typeof out2).to.not.equal('undefined');
    expect(typeof out3).to.not.equal('undefined');
  });

  it('name resolution and registration forward correctly', async () => {
    const raw = {};
    const sdk = EvoSDK.fromWasm(raw);
    await sdk.dpns.isNameAvailable('label');
    await sdk.dpns.resolveName('name');
    await sdk.dpns.registerName({ label: 'l', identityId: 'i', publicKeyId: 1, privateKeyWif: 'w' });
    await sdk.dpns.usernames('i', { limit: 2 });
    await sdk.dpns.username('i');
    await sdk.dpns.usernamesWithProof('i', { limit: 3 });
    await sdk.dpns.usernameWithProof('i');
    await sdk.dpns.getUsernameByName('u');
    await sdk.dpns.getUsernameByNameWithProof('u');

    const calls = wasmStubModule.__getCalls();
    const names = calls.map(c => c.called);
    expect(names).to.include.members([
      'dpns_is_name_available',
      'dpns_resolve_name',
      'dpns_register_name',
      'get_dpns_usernames',
      'get_dpns_username',
      'get_dpns_usernames_with_proof_info',
      'get_dpns_username_with_proof_info',
      'get_dpns_username_by_name',
      'get_dpns_username_by_name_with_proof_info',
    ]);
  });
});

