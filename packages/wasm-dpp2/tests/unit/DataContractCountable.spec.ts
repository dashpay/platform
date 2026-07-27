/**
 * Verifies that the v12-introduced `documentsCountable` / `rangeCountable`
 * top-level document-schema flags survive round-tripping through the JS
 * DataContract API. These flags select the primary-key tree variant in
 * Drive (NormalTree / CountTree / ProvableCountTree); if the JS layer
 * silently dropped them on serialize/deserialize, contracts created via
 * the SDK would store with the wrong tree shape.
 */
import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

let PlatformVersion: typeof wasm.PlatformVersion;

before(async () => {
  await initWasm();
  ({ PlatformVersion } = wasm);
});

const ownerId = '11111111111111111111111111111111';

function widgetSchemas(extras: Record<string, unknown>): Record<string, object> {
  return {
    widget: {
      type: 'object',
      properties: {
        name: { type: 'string', position: 0, maxLength: 64 },
      },
      additionalProperties: false,
      ...extras,
    },
  };
}

describe('DataContract — countable flags (v12)', () => {
  it('preserves documentsCountable on the schemas getter', () => {
    const dataContract = new wasm.DataContract({
      ownerId,
      identityNonce: BigInt(2),
      schemas: widgetSchemas({ documentsCountable: true }),
      definitions: null,
      fullValidation: true,
      platformVersion: new PlatformVersion(12),
    });

    const schemas = dataContract.schemas as Record<string, { documentsCountable?: boolean }>;
    expect(schemas.widget.documentsCountable).to.equal(true);
  });

  it('preserves rangeCountable on the schemas getter', () => {
    const dataContract = new wasm.DataContract({
      ownerId,
      identityNonce: BigInt(2),
      schemas: widgetSchemas({ rangeCountable: true }),
      definitions: null,
      fullValidation: true,
      platformVersion: new PlatformVersion(12),
    });

    const schemas = dataContract.schemas as Record<string, { rangeCountable?: boolean }>;
    expect(schemas.widget.rangeCountable).to.equal(true);
  });

  it('round-trips documentsCountable through toBytes / fromBytes', () => {
    const original = new wasm.DataContract({
      ownerId,
      identityNonce: BigInt(2),
      schemas: widgetSchemas({ documentsCountable: true }),
      definitions: null,
      fullValidation: true,
      platformVersion: new PlatformVersion(12),
    });

    const bytes = original.toBytes(new PlatformVersion(12));
    const restored = wasm.DataContract.fromBytes(bytes, true, new PlatformVersion(12));

    const schemas = restored.schemas as Record<string, { documentsCountable?: boolean }>;
    expect(schemas.widget.documentsCountable).to.equal(true);
  });

  it('round-trips rangeCountable through toBytes / fromBytes', () => {
    const original = new wasm.DataContract({
      ownerId,
      identityNonce: BigInt(2),
      schemas: widgetSchemas({ rangeCountable: true }),
      definitions: null,
      fullValidation: true,
      platformVersion: new PlatformVersion(12),
    });

    const bytes = original.toBytes(new PlatformVersion(12));
    const restored = wasm.DataContract.fromBytes(bytes, true, new PlatformVersion(12));

    const schemas = restored.schemas as Record<string, { rangeCountable?: boolean }>;
    expect(schemas.widget.rangeCountable).to.equal(true);
  });

  it('round-trips documentsCountable through toObject / fromObject', () => {
    const original = new wasm.DataContract({
      ownerId,
      identityNonce: BigInt(2),
      schemas: widgetSchemas({ documentsCountable: true }),
      definitions: null,
      fullValidation: true,
      platformVersion: new PlatformVersion(12),
    });

    const obj = original.toObject(new PlatformVersion(12));
    const restored = wasm.DataContract.fromObject(obj, true, new PlatformVersion(12));

    const schemas = restored.schemas as Record<string, { documentsCountable?: boolean }>;
    expect(schemas.widget.documentsCountable).to.equal(true);
  });

  it('full validation accepts documentsCountable + rangeCountable together at v12', () => {
    expect(() => {
      // eslint-disable-next-line no-new
      new wasm.DataContract({
        ownerId,
        identityNonce: BigInt(2),
        schemas: widgetSchemas({ documentsCountable: true, rangeCountable: true }),
        definitions: null,
        fullValidation: true,
        platformVersion: new PlatformVersion(12),
      });
    }).to.not.throw();
  });
});
