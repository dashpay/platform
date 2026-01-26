import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

/**
 * Tests for JavaScript value to JSON conversion via testJsValueToJson.
 *
 * These tests verify that normalize_js_value_for_json correctly handles
 * various JavaScript types for JSON serialization:
 *
 * - Maps: converted to plain objects (keys become strings)
 * - BigInt: converted to strings (JSON doesn't support BigInt)
 * - Uint8Array: converted to regular arrays
 * - WASM objects: their toJSON() method is called
 * - Nested structures: recursively normalized
 * - Primitives: passed through unchanged
 */

describe('JS value to JSON conversion (testJsValueToJson)', () => {
  before(async () => {
    await initWasm();
  });

  describe('Map with string keys', () => {
    it('should serialize empty Map to empty object', () => {
      const emptyMap = new Map();

      const json = wasm.testJsValueToJson(emptyMap);

      expect(json).to.deep.equal({});
    });

    it('should serialize Map with string keys and primitive values', () => {
      const map = new Map([
        ['key1', 'value1'],
        ['key2', 42],
        ['key3', true],
        ['key4', null],
      ]);

      const json = wasm.testJsValueToJson(map);

      expect(json).to.deep.equal({
        key1: 'value1',
        key2: 42,
        key3: true,
        key4: null,
      });
    });

    it('should serialize Map with string keys and object values', () => {
      const map = new Map([
        ['epoch1', { index: 1, startTime: 1000 }],
        ['epoch2', { index: 2, startTime: 2000 }],
      ]);

      const json = wasm.testJsValueToJson(map);

      expect(json).to.deep.equal({
        epoch1: { index: 1, startTime: 1000 },
        epoch2: { index: 2, startTime: 2000 },
      });
    });

    it('should serialize Map with string keys and array values', () => {
      const map = new Map([
        ['items1', [1, 2, 3]],
        ['items2', ['a', 'b', 'c']],
      ]);

      const json = wasm.testJsValueToJson(map);

      expect(json).to.deep.equal({
        items1: [1, 2, 3],
        items2: ['a', 'b', 'c'],
      });
    });
  });

  describe('Map with numeric keys', () => {
    it('should serialize Map with number keys (converted to strings)', () => {
      // This simulates getEpochsInfo which returns Map<number, EpochInfo>
      const map = new Map([
        [0, { index: 0, name: 'epoch0' }],
        [1, { index: 1, name: 'epoch1' }],
        [100, { index: 100, name: 'epoch100' }],
      ]);

      const json = wasm.testJsValueToJson(map);

      // Keys should be strings in JSON
      expect(json).to.have.property('0');
      expect(json).to.have.property('1');
      expect(json).to.have.property('100');
      expect(json['0']).to.deep.equal({ index: 0, name: 'epoch0' });
      expect(json['1']).to.deep.equal({ index: 1, name: 'epoch1' });
      expect(json['100']).to.deep.equal({ index: 100, name: 'epoch100' });
    });

    it('should serialize Map with negative number keys', () => {
      const map = new Map([
        [-1, 'negative one'],
        [0, 'zero'],
        [1, 'one'],
      ]);

      const json = wasm.testJsValueToJson(map);

      expect(json).to.have.property('-1');
      expect(json).to.have.property('0');
      expect(json).to.have.property('1');
      expect(json['-1']).to.equal('negative one');
    });

    it('should serialize Map with floating point keys', () => {
      const map = new Map([
        [1.5, 'one point five'],
        [2.7, 'two point seven'],
      ]);

      const json = wasm.testJsValueToJson(map);

      expect(json).to.have.property('1.5');
      expect(json).to.have.property('2.7');
    });
  });

  describe('Map with BigInt keys', () => {
    it('should serialize Map with BigInt keys (converted to strings)', () => {
      const map = new Map([
        [1n, 'one'],
        [9007199254740993n, 'large number'], // > MAX_SAFE_INTEGER
      ]);

      const json = wasm.testJsValueToJson(map);

      expect(json).to.have.property('1');
      expect(json).to.have.property('9007199254740993');
      expect(json['1']).to.equal('one');
      expect(json['9007199254740993']).to.equal('large number');
    });
  });

  describe('Map with BigInt values', () => {
    it('should serialize Map with BigInt values (converted to strings)', () => {
      // This simulates token balances or credit amounts
      const map = new Map([
        ['balance1', 1000000000n],
        ['balance2', 9007199254740993n], // > MAX_SAFE_INTEGER
      ]);

      const json = wasm.testJsValueToJson(map);

      expect(json.balance1).to.equal('1000000000');
      expect(json.balance2).to.equal('9007199254740993');
    });
  });

  describe('Map with Identifier keys', () => {
    it('should serialize Map with Identifier keys using toString()', () => {
      const id1 = wasm.Identifier.fromBase58('H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1');
      const id2 = wasm.Identifier.fromBase58('ckBqfQe7LU7vwrwXopyCB4n5phZShjA16BGhNGpsD5U');

      // This simulates getEvonodesProposedEpochBlocksByIds which returns Map<Identifier, bigint>
      const map = new Map([
        [id1, 100n],
        [id2, 200n],
      ]);

      const json = wasm.testJsValueToJson(map);

      // Keys should be the string representation of Identifier (Base58)
      expect(json).to.have.property('H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1');
      expect(json).to.have.property('ckBqfQe7LU7vwrwXopyCB4n5phZShjA16BGhNGpsD5U');
      // Values are BigInt, should be strings
      expect(json.H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1).to.equal('100');
      expect(json.ckBqfQe7LU7vwrwXopyCB4n5phZShjA16BGhNGpsD5U).to.equal('200');
    });
  });

  describe('Map with WASM object values', () => {
    it('should serialize Map with Identity values by calling their toJSON()', () => {
      const identity = new wasm.Identity('H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1');
      identity.balance = 1000000000n;
      identity.revision = 5n;

      const map = new Map([['identity1', identity]]);

      const json = wasm.testJsValueToJson(map);

      expect(json).to.have.property('identity1');
      expect(json.identity1).to.have.property('id');
      expect(json.identity1.id).to.equal('H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1');
      expect(json.identity1.balance).to.equal(1000000000);
      expect(json.identity1.revision).to.equal(5);
    });

    it('should serialize Map with Identifier values', () => {
      const id1 = wasm.Identifier.fromBase58('H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1');
      const id2 = wasm.Identifier.fromBase58('ckBqfQe7LU7vwrwXopyCB4n5phZShjA16BGhNGpsD5U');

      const map = new Map([
        ['id1', id1],
        ['id2', id2],
      ]);

      const json = wasm.testJsValueToJson(map);

      // Identifier.toJSON() returns base58 string
      expect(json.id1).to.equal('H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1');
      expect(json.id2).to.equal('ckBqfQe7LU7vwrwXopyCB4n5phZShjA16BGhNGpsD5U');
    });

    it('should serialize Map with undefined/null values', () => {
      // This simulates queries that return Map<key, value | undefined>
      const map = new Map<number, { epoch: number } | undefined | null>([
        [0, { epoch: 0 }],
        [1, undefined],
        [2, null],
        [3, { epoch: 3 }],
      ]);

      const json = wasm.testJsValueToJson(map);

      expect(json['0']).to.deep.equal({ epoch: 0 });
      // Note: In JSON, undefined becomes null (JSON doesn't have undefined)
      expect(json['1']).to.satisfy((v: unknown) => v === null || v === undefined);
      expect(json['2']).to.equal(null);
      expect(json['3']).to.deep.equal({ epoch: 3 });
    });
  });

  describe('Nested Maps', () => {
    it('should serialize nested Map as value', () => {
      const innerMap = new Map([
        ['a', 1],
        ['b', 2],
      ]);
      const outerMap = new Map([['inner', innerMap]]);

      const json = wasm.testJsValueToJson(outerMap);

      expect(json).to.have.property('inner');
      expect(json.inner).to.deep.equal({ a: 1, b: 2 });
    });

    it('should serialize deeply nested Maps', () => {
      const level3 = new Map([['deep', 'value']]);
      const level2 = new Map([['level3', level3]]);
      const level1 = new Map([['level2', level2]]);

      const json = wasm.testJsValueToJson(level1);

      expect(json.level2.level3.deep).to.equal('value');
    });

    it('should serialize Map inside array inside Map', () => {
      const innerMap = new Map([['key', 'value']]);
      const outerMap = new Map([['items', [innerMap, { plain: 'object' }]]]);

      const json = wasm.testJsValueToJson(outerMap);

      expect(json.items[0]).to.deep.equal({ key: 'value' });
      expect(json.items[1]).to.deep.equal({ plain: 'object' });
    });
  });

  describe('Map inside array', () => {
    it('should serialize array containing Maps', () => {
      const map1 = new Map([['a', 1]]);
      const map2 = new Map([['b', 2]]);
      const arrayWithMaps = [map1, map2];

      const json = wasm.testJsValueToJson(arrayWithMaps);

      expect(json).to.be.an('array');
      expect(json[0]).to.deep.equal({ a: 1 });
      expect(json[1]).to.deep.equal({ b: 2 });
    });
  });

  describe('Map inside object', () => {
    it('should serialize object containing Map property', () => {
      const map = new Map([['key', 'value']]);
      const objectWithMap = {
        normalProp: 'normal',
        mapProp: map,
      };

      const json = wasm.testJsValueToJson(objectWithMap);

      expect(json.normalProp).to.equal('normal');
      expect(json.mapProp).to.deep.equal({ key: 'value' });
    });
  });

  describe('Map with Uint8Array values', () => {
    it('should serialize Map with Uint8Array values to arrays', () => {
      const map = new Map([
        ['bytes1', new Uint8Array([1, 2, 3])],
        ['bytes2', new Uint8Array([4, 5, 6])],
      ]);

      const json = wasm.testJsValueToJson(map);

      // Uint8Array should be converted to regular arrays for JSON
      expect(json.bytes1).to.deep.equal([1, 2, 3]);
      expect(json.bytes2).to.deep.equal([4, 5, 6]);
    });
  });

  describe('Real-world epoch query simulation', () => {
    it('should serialize epoch info Map like getEpochsInfoWithProofInfo returns', () => {
      // Simulating the structure returned by getEpochsInfoWithProofInfo
      const epochsMap = new Map<
        number,
        | {
            index: number;
            firstBlockHeight: bigint;
            firstCoreBlockHeight: number;
            startTime: bigint;
            feeMultiplier: number;
          }
        | undefined
      >([
        [
          0,
          {
            index: 0,
            firstBlockHeight: 1n,
            firstCoreBlockHeight: 100,
            startTime: 1609459200000n,
            feeMultiplier: 1.0,
          },
        ],
        [
          1,
          {
            index: 1,
            firstBlockHeight: 1000n,
            firstCoreBlockHeight: 200,
            startTime: 1609545600000n,
            feeMultiplier: 1.1,
          },
        ],
        [2, undefined], // Some epochs might not exist
      ]);

      const json = wasm.testJsValueToJson(epochsMap);

      expect(json).to.have.property('0');
      expect(json).to.have.property('1');
      expect(json).to.have.property('2');

      expect(json['0'].index).to.equal(0);
      expect(json['0'].firstBlockHeight).to.equal('1'); // BigInt -> string
      expect(json['0'].startTime).to.equal('1609459200000'); // BigInt -> string

      expect(json['1'].index).to.equal(1);
      // Note: In JSON, undefined becomes null (JSON doesn't have undefined)
      expect(json['2']).to.satisfy((v: unknown) => v === null || v === undefined);
    });
  });

  describe('Edge cases', () => {
    it('should handle Map with special string keys', () => {
      const map = new Map([
        ['', 'empty key'],
        ['with spaces', 'spaced'],
        ['with.dots', 'dotted'],
        ['with/slash', 'slashed'],
        ['with\\backslash', 'backslashed'],
        ['unicode: \u00e9', 'accented'],
      ]);

      const json = wasm.testJsValueToJson(map);

      expect(json['']).to.equal('empty key');
      expect(json['with spaces']).to.equal('spaced');
      expect(json['with.dots']).to.equal('dotted');
      expect(json['with/slash']).to.equal('slashed');
      expect(json['with\\backslash']).to.equal('backslashed');
      expect(json['unicode: \u00e9']).to.equal('accented');
    });

    it('should preserve key order in Map', () => {
      const map = new Map([
        ['z', 1],
        ['a', 2],
        ['m', 3],
      ]);

      const json = wasm.testJsValueToJson(map);
      const keys = Object.keys(json);

      // Maps preserve insertion order
      expect(keys).to.deep.equal(['z', 'a', 'm']);
    });

    it('should handle large Maps', () => {
      const map = new Map();
      for (let i = 0; i < 1000; i += 1) {
        map.set(i, { index: i, value: `item${i}` });
      }

      const json = wasm.testJsValueToJson(map);

      expect(Object.keys(json).length).to.equal(1000);
      expect(json['0']).to.deep.equal({ index: 0, value: 'item0' });
      expect(json['999']).to.deep.equal({ index: 999, value: 'item999' });
    });
  });

  describe('Primitive values', () => {
    it('should pass through strings', () => {
      const result = wasm.testJsValueToJson('hello');
      expect(result).to.equal('hello');
    });

    it('should pass through numbers', () => {
      const result = wasm.testJsValueToJson(42);
      expect(result).to.equal(42);
    });

    it('should pass through booleans', () => {
      expect(wasm.testJsValueToJson(true)).to.equal(true);
      expect(wasm.testJsValueToJson(false)).to.equal(false);
    });

    it('should pass through null', () => {
      const result = wasm.testJsValueToJson(null);
      expect(result).to.equal(null);
    });

    it('should convert BigInt to string', () => {
      const result = wasm.testJsValueToJson(9007199254740993n);
      expect(result).to.equal('9007199254740993');
    });

    it('should convert Uint8Array to array', () => {
      const result = wasm.testJsValueToJson(new Uint8Array([1, 2, 3]));
      expect(result).to.deep.equal([1, 2, 3]);
    });
  });

  describe('Plain objects and arrays', () => {
    it('should pass through plain objects', () => {
      const obj = { a: 1, b: 'two', c: true };
      const result = wasm.testJsValueToJson(obj);
      expect(result).to.deep.equal(obj);
    });

    it('should pass through arrays', () => {
      const arr = [1, 'two', true, null];
      const result = wasm.testJsValueToJson(arr);
      expect(result).to.deep.equal(arr);
    });

    it('should recursively handle nested objects with BigInt', () => {
      const obj = {
        outer: {
          inner: {
            value: 9007199254740993n,
          },
        },
      };
      const result = wasm.testJsValueToJson(obj);
      expect(result.outer.inner.value).to.equal('9007199254740993');
    });

    it('should recursively handle arrays with BigInt', () => {
      const arr = [1n, 2n, 9007199254740993n];
      const result = wasm.testJsValueToJson(arr);
      expect(result).to.deep.equal(['1', '2', '9007199254740993']);
    });
  });
});
