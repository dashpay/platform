import init, * as sdk from '../../dist/sdk.js';

describe('DPNS utils (homograph + validation)', () => {
  before(async () => {
    await init();
  });
  it('convert to homograph safe retains allowed chars', () => {
    expect(sdk.WasmSdk.dpnsConvertToHomographSafe('test')).to.equal('test');
    expect(sdk.WasmSdk.dpnsConvertToHomographSafe('test123')).to.equal('test123');
    expect(sdk.WasmSdk.dpnsConvertToHomographSafe('test-name')).to.equal('test-name');
    expect(sdk.WasmSdk.dpnsConvertToHomographSafe('TestName')).to.equal('testname');
  });

  it('homograph conversions for o,i,l to 0/1/1', () => {
    const input = 'IlIooLi';
    expect(sdk.WasmSdk.dpnsConvertToHomographSafe(input)).to.equal('1110011');
  });

  it('preserves non-homograph unicode (lowercased)', () => {
    expect(sdk.WasmSdk.dpnsConvertToHomographSafe('tеst')).to.equal('tеst');
  });

  it('username validation basic rules', () => {
    expect(sdk.WasmSdk.dpnsIsValidUsername('alice')).to.equal(true);
    expect(sdk.WasmSdk.dpnsIsValidUsername('alice123')).to.equal(true);
    expect(sdk.WasmSdk.dpnsIsValidUsername('alice-bob')).to.equal(true);
    expect(sdk.WasmSdk.dpnsIsValidUsername('ab')).to.equal(false);
    expect(sdk.WasmSdk.dpnsIsValidUsername('a'.repeat(64))).to.equal(false);
    expect(sdk.WasmSdk.dpnsIsValidUsername('-alice')).to.equal(false);
    expect(sdk.WasmSdk.dpnsIsValidUsername('alice-')).to.equal(false);
    expect(sdk.WasmSdk.dpnsIsValidUsername('alice--bob')).to.equal(false);
  });
});
