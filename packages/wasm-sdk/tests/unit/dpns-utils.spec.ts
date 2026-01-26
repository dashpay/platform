import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

describe('WasmSdk', () => {
  before(async () => {
    await init();
  });

  describe('dpnsConvertToHomographSafe()', () => {
    it('should retain lowercase letters', () => {
      expect(sdk.WasmSdk.dpnsConvertToHomographSafe('test')).to.equal('test');
    });

    it('should retain numbers', () => {
      expect(sdk.WasmSdk.dpnsConvertToHomographSafe('test123')).to.equal('test123');
    });

    it('should retain hyphens', () => {
      expect(sdk.WasmSdk.dpnsConvertToHomographSafe('test-name')).to.equal('test-name');
    });

    it('should convert uppercase to lowercase', () => {
      expect(sdk.WasmSdk.dpnsConvertToHomographSafe('TestName')).to.equal('testname');
    });

    it('should convert homographs o, i, l to 0/1/1', () => {
      const input = 'IlIooLi';
      expect(sdk.WasmSdk.dpnsConvertToHomographSafe(input)).to.equal('1110011');
    });

    it('should preserve non-homograph unicode (lowercased)', () => {
      expect(sdk.WasmSdk.dpnsConvertToHomographSafe('tеst')).to.equal('tеst');
    });
  });

  describe('dpnsIsValidUsername()', () => {
    it('should accept valid lowercase username', () => {
      expect(sdk.WasmSdk.dpnsIsValidUsername('alice')).to.equal(true);
    });

    it('should accept username with numbers', () => {
      expect(sdk.WasmSdk.dpnsIsValidUsername('alice123')).to.equal(true);
    });

    it('should accept username with hyphen in middle', () => {
      expect(sdk.WasmSdk.dpnsIsValidUsername('alice-bob')).to.equal(true);
    });

    it('should reject username shorter than 3 characters', () => {
      expect(sdk.WasmSdk.dpnsIsValidUsername('ab')).to.equal(false);
    });

    it('should reject username longer than 63 characters', () => {
      expect(sdk.WasmSdk.dpnsIsValidUsername('a'.repeat(64))).to.equal(false);
    });

    it('should reject username starting with hyphen', () => {
      expect(sdk.WasmSdk.dpnsIsValidUsername('-alice')).to.equal(false);
    });

    it('should reject username ending with hyphen', () => {
      expect(sdk.WasmSdk.dpnsIsValidUsername('alice-')).to.equal(false);
    });

    it('should reject username with consecutive hyphens', () => {
      expect(sdk.WasmSdk.dpnsIsValidUsername('alice--bob')).to.equal(false);
    });
  });
});
