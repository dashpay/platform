import { asJsonString, generateEntropy } from '../../dist/util.js';

describe('Util Functions', () => {
  describe('asJsonString', () => {
    it('returns undefined for null', () => {
      expect(asJsonString(null)).to.be.undefined();
    });

    it('returns undefined for undefined', () => {
      expect(asJsonString(undefined)).to.be.undefined();
    });

    it('returns string as-is', () => {
      expect(asJsonString('hello')).to.equal('hello');
    });

    it('converts objects to JSON string', () => {
      const obj = { foo: 'bar', num: 42 };
      expect(asJsonString(obj)).to.equal(JSON.stringify(obj));
    });

    it('converts arrays to JSON string', () => {
      const arr = [1, 2, 'three'];
      expect(asJsonString(arr)).to.equal(JSON.stringify(arr));
    });
  });

  describe('generateEntropy', () => {
    it('generates a 64-character hex string', () => {
      const entropy = generateEntropy();
      expect(entropy).to.be.a('string');
      expect(entropy.length).to.equal(64);
    });

    it('generates valid hexadecimal', () => {
      const entropy = generateEntropy();
      expect(entropy).to.match(/^[0-9a-f]{64}$/i);
    });

    it('generates different values each time', () => {
      const entropy1 = generateEntropy();
      const entropy2 = generateEntropy();
      const entropy3 = generateEntropy();

      // Should be different (extremely unlikely to be the same)
      expect(entropy1).to.not.equal(entropy2);
      expect(entropy2).to.not.equal(entropy3);
      expect(entropy1).to.not.equal(entropy3);
    });

    it('returns exactly 32 bytes when decoded', () => {
      const entropy = generateEntropy();
      // Convert hex string to bytes
      const bytes = [];
      for (let i = 0; i < entropy.length; i += 2) {
        bytes.push(parseInt(entropy.substring(i, 2), 16));
      }
      expect(bytes.length).to.equal(32);
    });

    it('generates values with good distribution', () => {
      // Generate multiple samples and check that we get a variety of hex digits
      const samples = [];
      for (let i = 0; i < 10; i += 1) {
        samples.push(generateEntropy());
      }

      // Check that we see various hex digits (not all zeros or all ones)
      const allChars = samples.join('');
      const uniqueChars = new Set(allChars).size;

      // We should see most of the 16 possible hex digits (0-9, a-f)
      // With 640 characters (10 * 64), we expect to see all 16
      expect(uniqueChars).to.be.at.least(10);
    });
  });
});
