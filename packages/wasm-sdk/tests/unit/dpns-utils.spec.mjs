import chai from 'chai';
import init, * as sdk from '../../dist/sdk.js';

describe('DPNS utils (homograph + validation)', () => {
  before(async () => {
    await init();
  });
  it('convert to homograph safe retains allowed chars', () => {
    expect(sdk.dpns_convert_to_homograph_safe('test')).to.equal('test');
    expect(sdk.dpns_convert_to_homograph_safe('test123')).to.equal('test123');
    expect(sdk.dpns_convert_to_homograph_safe('test-name')).to.equal('test-name');
    expect(sdk.dpns_convert_to_homograph_safe('TestName')).to.equal('testname');
  });

  it('homograph conversions for o,i,l to 0/1/1', () => {
    const input = 'IlIooLi';
    expect(sdk.dpns_convert_to_homograph_safe(input)).to.equal('1110011');
  });

  it('preserves non-homograph unicode (lowercased)', () => {
    expect(sdk.dpns_convert_to_homograph_safe('tеst')).to.equal('tеst');
  });

  it('username validation basic rules', () => {
    expect(sdk.dpns_is_valid_username('alice')).to.equal(true);
    expect(sdk.dpns_is_valid_username('alice123')).to.equal(true);
    expect(sdk.dpns_is_valid_username('alice-bob')).to.equal(true);
    expect(sdk.dpns_is_valid_username('ab')).to.equal(false);
    expect(sdk.dpns_is_valid_username('a'.repeat(64))).to.equal(false);
    expect(sdk.dpns_is_valid_username('-alice')).to.equal(false);
    expect(sdk.dpns_is_valid_username('alice-')).to.equal(false);
    expect(sdk.dpns_is_valid_username('alice--bob')).to.equal(false);
  });
});
