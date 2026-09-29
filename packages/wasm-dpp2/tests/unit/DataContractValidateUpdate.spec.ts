/**
 * `DataContract.validateUpdate`: the contract update rules consensus runs on a
 * data contract update transition (`DataContract::validate_update`), without
 * reading state. It returns the consensus errors a refused update would carry,
 * or an empty array for a valid one.
 */
import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

let PlatformVersion: typeof wasm.PlatformVersion;

before(async () => {
  await initWasm();
  ({ PlatformVersion } = wasm);
});

const contractId = '4fJLR2GYTPFdomuTVvNy3VRrvWgvkKPzqehEBpNf2nk6';
const ownerId = 'CXH2kZCATjvDTnQAPVg28EgPg9WySUvwvnR5ZkmNqY5i';
const otherOwnerId = '9tSsCqKHTZ8ro16MydChSxgHBukFW36eMLJKKRtebJEn';

type Json = Record<string, any>;

/** Version 1 of a small contract: a `note` with one index, and a `tag`. */
function baseJson(): Json {
  return {
    $formatVersion: '1',
    id: contractId,
    ownerId,
    version: 1,
    documentSchemas: {
      note: {
        type: 'object',
        properties: {
          title: { type: 'string', maxLength: 63, position: 0 },
          body: { type: 'string', maxLength: 1000, position: 1 },
        },
        required: ['title'],
        indices: [{ name: 'byTitle', properties: [{ title: 'asc' }] }],
        additionalProperties: false,
      },
      tag: {
        type: 'object',
        properties: {
          label: { type: 'string', maxLength: 32, position: 0 },
        },
        additionalProperties: false,
      },
    },
  };
}

/** Version 2: a copy of version 1 for a test to change. */
function nextJson(): Json {
  return { ...structuredClone(baseJson()), version: 2 };
}

function contract(value: Json) {
  return wasm.DataContract.fromJSON(value, true, new PlatformVersion(14));
}

function codesOf(next: Json, blockInfo?: InstanceType<typeof wasm.BlockInfo>) {
  return contract(baseJson())
    .validateUpdate(contract(next), blockInfo, new PlatformVersion(14))
    .map((error: InstanceType<typeof wasm.ConsensusError>) => error.code);
}

describe('DataContract.validateUpdate()', () => {
  it('should accept an update that adds an optional property and raises the version by one', () => {
    const next = nextJson();
    next.documentSchemas.note.properties.color = { type: 'string', maxLength: 16, position: 2 };

    expect(codesOf(next)).to.deep.equal([]);
  });

  it('should accept raising maxLength and adding a document type', () => {
    const next = nextJson();
    next.documentSchemas.note.properties.body.maxLength = 2000;
    next.documentSchemas.folder = {
      type: 'object',
      properties: { name: { type: 'string', maxLength: 32, position: 0 } },
      additionalProperties: false,
    };

    expect(codesOf(next)).to.deep.equal([]);
  });

  it('should return ConsensusError objects with the code and message consensus reports', () => {
    const next = nextJson();
    next.version = 3;

    const [error] = contract(baseJson()).validateUpdate(contract(next), undefined, new PlatformVersion(14));

    expect(error).to.be.an.instanceof(wasm.ConsensusError);
    expect(error.code).to.equal(10212); // InvalidDataContractVersionError
    expect(error.message).to.be.a('string').and.not.be.empty();
  });

  it('should refuse an update that keeps the version', () => {
    const next = nextJson();
    next.version = 1;
    next.documentSchemas.note.properties.color = { type: 'string', maxLength: 16, position: 2 };

    expect(codesOf(next)).to.deep.equal([10212]); // InvalidDataContractVersionError
  });

  it('should refuse a change of owner', () => {
    const next = nextJson();
    next.ownerId = otherOwnerId;

    expect(codesOf(next)).to.deep.equal([40003]); // DataContractUpdatePermissionError
  });

  it('should refuse removing a document type', () => {
    const next = nextJson();
    delete next.documentSchemas.tag;

    expect(codesOf(next)).to.deep.equal([40212]); // DocumentTypeUpdateError
  });

  it('should refuse adding an index to an existing document type', () => {
    const next = nextJson();
    next.documentSchemas.note.indices.push({ name: 'byOwner', properties: [{ $ownerId: 'asc' }] });

    expect(codesOf(next)).to.deep.equal([10217]); // DataContractInvalidIndexDefinitionUpdateError
  });

  it('should refuse removing a property', () => {
    const next = nextJson();
    delete next.documentSchemas.note.properties.body;

    expect(codesOf(next)).to.deep.equal([10246]); // IncompatibleDocumentTypeSchemaError
  });

  it('should refuse lowering maxLength', () => {
    const next = nextJson();
    next.documentSchemas.note.properties.body.maxLength = 500;

    expect(codesOf(next)).to.deep.equal([10246]); // IncompatibleDocumentTypeSchemaError
  });

  it('should refuse a new required property without requiredSince, and accept it with requiredSince', () => {
    const without = nextJson();
    without.documentSchemas.tag.properties.color = { type: 'string', maxLength: 16, position: 1 };
    without.documentSchemas.tag.required = ['color'];

    expect(codesOf(without)).to.deep.equal([10276]); // DataContractInvalidRequiredFieldsUpdateError

    const withSince = structuredClone(without);
    withSince.documentSchemas.tag.properties.color.requiredSince = 2;

    expect(codesOf(withSince)).to.deep.equal([]);
  });

  it('should run the document meta-schema in full validation (this build has `validation`)', () => {
    const typo = baseJson();
    typo.documentSchemas.tag.documentsMutible = false; // not a keyword

    expect(() => contract(typo)).to.throw();
  });

  it('should take a block info without consuming it', () => {
    const blockInfo = new wasm.BlockInfo({
      timeMs: 1790000000000n, height: 10n, coreHeight: 5, epochIndex: 1,
    });
    const next = nextJson();

    expect(codesOf(next, blockInfo)).to.deep.equal([]);
    expect(blockInfo.timeMs).to.equal(1790000000000n);
  });
});
