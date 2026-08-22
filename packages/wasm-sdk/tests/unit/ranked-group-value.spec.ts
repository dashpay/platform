/**
 * Regression coverage for the `groupValue` conversion on ranked and
 * having-range results.
 *
 * A group key decodes to whatever the indexed property's declared type is,
 * and `u64` / `i64` / `u128` / `i128` properties are all reachable (a `Date`
 * group key decodes to `u64` too). Routing those through the JSON
 * conversion targets a JS `number`, which *errors* past
 * `Number.MAX_SAFE_INTEGER` rather than rounding — so one large group key
 * used to reject an entire verified page.
 *
 * These cases live at the JS boundary, which a host-target Rust test cannot
 * reach, hence the `testRankedGroupValue` export.
 *
 * Scope note: the helper's input conversion normalizes every JS number to
 * `i64` and cannot represent a BigInt outside the `i64::MIN..u64::MAX`
 * range, so the narrower integer types (and `u128` / `i128`) are not
 * expressible here. Their classification is covered by the Rust host tests
 * in `document_ranked.rs`; what this spec pins is the boundary behaviour
 * those tests cannot observe.
 */
import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

describe('ranked groupValue conversion', () => {
  before(async () => {
    await init();
  });

  describe('integers past the safe-integer range', () => {
    it('should return an exact bigint just above Number.MAX_SAFE_INTEGER', () => {
      const key = BigInt(Number.MAX_SAFE_INTEGER) + BigInt(2);

      const groupValue = sdk.testRankedGroupValue(key);

      expect(typeof groupValue).to.equal('bigint');
      expect(groupValue).to.equal(key);
    });

    it('should return an exact bigint at the top of the u64 range', () => {
      // A `u64` group key can legitimately reach here — token amounts and
      // credit balances are the obvious cases.
      const key = BigInt('18446744073709551615');

      const groupValue = sdk.testRankedGroupValue(key);

      expect(groupValue).to.equal(key);
    });

    it('should return an exact bigint for a large negative i64 key', () => {
      const key = -(BigInt(Number.MAX_SAFE_INTEGER) + BigInt(2));

      const groupValue = sdk.testRankedGroupValue(key);

      expect(groupValue).to.equal(key);
    });

    it('should not throw for a key that has no lossless number representation', () => {
      // The reported failure: the whole page rejected because one group's
      // key was large. Assert the absence of the throw explicitly.
      expect(() => sdk.testRankedGroupValue(BigInt('9007199254740993'))).to.not.throw();
    });
  });

  describe('everything else keeps the document JSON convention', () => {
    it('should return a string group key as a string', () => {
      expect(sdk.testRankedGroupValue('alice')).to.equal('alice');
    });

    it('should return a boolean group key as a boolean', () => {
      expect(sdk.testRankedGroupValue(true)).to.equal(true);
    });
  });
});
