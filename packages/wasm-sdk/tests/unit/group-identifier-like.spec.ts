import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';
import contractFixtureV0 from './fixtures/data-contract-v0-crypto-card-game.ts';

type IdentifierForm = 'instance' | 'bytes' | 'base58' | 'hex' | 'array';
type MakeIdentifier = (byte: number, path: string) => unknown;
type QueryMethod =
  | 'getContractGroupMembers'
  | 'getContractGroupMembersWithProofInfo'
  | 'getGroupActions'
  | 'getGroupActionsWithProofInfo'
  | 'getGroupActionSigners'
  | 'getGroupActionSignersWithProofInfo'
  | 'getGroupInfos'
  | 'getGroupInfosWithProofInfo'
  | 'getGroupMembers'
  | 'getGroupMembersWithProofInfo'
  | 'getIdentityGroups'
  | 'getIdentityGroupsWithProofInfo';
type QueryCase = {
  name: string;
  method: QueryMethod;
  proofMethod: QueryMethod;
  paths: string[];
  query: (make: MakeIdentifier) => object;
  message?: string;
};

const cases: QueryCase[] = [
  {
    name: 'contract group members',
    method: 'getContractGroupMembers',
    proofMethod: 'getContractGroupMembersWithProofInfo',
    paths: ['contractGroupId', 'startAfter.contractId'],
    query: (make) => ({
      contractGroupId: make(0x11, 'contractGroupId'),
      kind: 'tokens',
      startAfter: { contractId: make(0x22, 'startAfter.contractId') },
      limit: 2,
    }),
    message: "startAfter.tokenPosition is required for kind 'tokens'",
  },
  {
    name: 'group actions',
    method: 'getGroupActions',
    proofMethod: 'getGroupActionsWithProofInfo',
    paths: ['dataContractId', 'startAt.actionId'],
    query: (make) => ({
      dataContractId: make(0x11, 'dataContractId'),
      groupContractPosition: 7,
      status: 'INVALID',
      startAt: { actionId: make(0x22, 'startAt.actionId'), included: true },
      limit: 2,
    }),
    message: 'Invalid status: INVALID. Must be ACTIVE or CLOSED',
  },
  {
    name: 'group action signers',
    method: 'getGroupActionSigners',
    proofMethod: 'getGroupActionSignersWithProofInfo',
    paths: ['dataContractId', 'actionId'],
    query: (make) => ({
      dataContractId: make(0x11, 'dataContractId'),
      groupContractPosition: 7,
      status: 'INVALID',
      actionId: make(0x22, 'actionId'),
    }),
    message: 'Invalid status: INVALID. Must be ACTIVE or CLOSED',
  },
  {
    name: 'group infos',
    method: 'getGroupInfos',
    proofMethod: 'getGroupInfosWithProofInfo',
    paths: ['dataContractId'],
    query: (make) => ({
      dataContractId: make(0x11, 'dataContractId'),
      startAt: { position: 17, included: true },
      limit: 70000,
    }),
    message: 'limit 70000 exceeds maximum of 65535',
  },
  {
    name: 'group members',
    method: 'getGroupMembers',
    proofMethod: 'getGroupMembersWithProofInfo',
    paths: ['dataContractId', 'memberIds[0]', 'memberIds[1]', 'startAtMemberId'],
    query: (make) => ({
      dataContractId: make(0x11, 'dataContractId'),
      groupContractPosition: 7,
      memberIds: [make(0x22, 'memberIds[0]'), make(0x33, 'memberIds[1]')],
      startAtMemberId: make(0x44, 'startAtMemberId'),
      limit: 70000,
    }),
    message: 'limit 70000 exceeds maximum of 65535',
  },
  {
    name: 'identity groups',
    method: 'getIdentityGroups',
    proofMethod: 'getIdentityGroupsWithProofInfo',
    paths: ['identityId', 'ownerDataContracts[0]', 'moderatorDataContracts[0]'],
    query: (make) => ({
      identityId: make(0x11, 'identityId'),
      memberDataContracts: [],
      ownerDataContracts: [make(0x22, 'ownerDataContracts[0]')],
      moderatorDataContracts: [make(0x33, 'moderatorDataContracts[0]')],
    }),
  },
];

function freezeContainers(value: unknown): void {
  if (Array.isArray(value) || (value && Object.getPrototypeOf(value) === Object.prototype)) {
    Object.values(value).forEach(freezeContainers);
    Object.freeze(value);
  }
}

describe('group query IdentifierLike inputs', () => {
  let subclassByteCalls: number;
  class CustomIdentifier extends sdk.Identifier {
    toBytes(): Uint8Array {
      subclassByteCalls += 1;
      throw new Error('the input override must not run');
    }
  }
  let client: sdk.WasmSdk;
  let otherClient: sdk.WasmSdk;
  let originalFetch: typeof globalThis.fetch;
  let fetchCalls: number;
  let identifiers: sdk.Identifier[];
  let errors: sdk.WasmSdkError[];

  before(async function setup() {
    this.timeout(120000);
    await init();
    client = await sdk.WasmSdkBuilder.withAddresses(['https://group-query.test:1443'], 'testnet')
      .withVersion(1)
      .build();
    otherClient = await sdk.WasmSdkBuilder.withAddresses(['https://other-group-query.test:1443'], 'testnet')
      .withVersion(1)
      .build();
  });

  beforeEach(() => {
    subclassByteCalls = 0;
    identifiers = [];
    errors = [];
    fetchCalls = 0;
    originalFetch = globalThis.fetch;
    globalThis.fetch = async () => {
      fetchCalls += 1;
      throw new Error('group query parsing must finish before fetching');
    };
  });

  afterEach(() => {
    try {
      expect(fetchCalls).to.equal(0);
    } finally {
      globalThis.fetch = originalFetch;
      errors.forEach((error) => error.free());
      identifiers.forEach((value) => value.free());
    }
  });

  after(() => {
    client?.free();
    otherClient?.free();
  });

  function identifier(byte: number, form: IdentifierForm): unknown {
    const bytes = new Uint8Array(32).fill(byte);
    if (form === 'bytes') {
      return bytes;
    }
    if (form === 'array') {
      return Array.from(bytes);
    }
    if (form === 'hex') {
      return Array.from(bytes, (value) => value.toString(16).padStart(2, '0')).join('');
    }
    const value = new sdk.Identifier(bytes);
    identifiers.push(value);
    return form === 'base58' ? value.toBase58() : value;
  }

  async function rejection(promise: Promise<unknown>): Promise<sdk.WasmSdkError> {
    try {
      await promise;
    } catch (error) {
      if (!(error instanceof sdk.WasmSdkError)) {
        throw error;
      }
      errors.push(error);
      expect(error.kind).to.equal(sdk.WasmSdkErrorKind.InvalidArgument);
      return error;
    }
    throw new Error('expected a query validation error');
  }

  function trackIdentifier(value: sdk.Identifier): void {
    identifiers.push(value);
  }

  function callQuery(query: object, method: QueryMethod, target = client): Promise<unknown> {
    const call = target[method] as (input: never) => Promise<unknown>;
    return call.call(target, query as never);
  }

  async function checkOutcome(testCase: QueryCase, proof: boolean, query: object, target = client) {
    freezeContainers(query);
    const method = proof ? testCase.proofMethod : testCase.method;
    const result = callQuery(query, method, target);
    if (testCase.message) {
      expect((await rejection(result)).message).to.equal(testCase.message);
    } else {
      const response = await result;
      if (proof) {
        const value = response as sdk.ProofMetadataResponse;
        try {
          expect(value.data).to.deep.equal([]);
        } finally {
          value.free();
        }
      } else {
        expect(response).to.deep.equal([]);
      }
    }
  }

  for (const [kind, message] of [
    ['contracts', 'limit 70000 exceeds maximum of 65535'],
    ['documentTypes', "startAfter.documentTypeName is required for kind 'documentTypes'"],
  ] as const) {
    for (const proof of [false, true]) {
      for (const form of ['bytes', 'instance'] as IdentifierForm[]) {
        it(`should preserve ${kind} cursor validation with ${form} identifiers and proof ${proof}`, async () => {
          const testCase = { ...cases[0], message };
          const query = {
            contractGroupId: identifier(0x11, form),
            kind,
            startAfter: { contractId: identifier(0x22, form) },
            limit: 70000,
          };
          await checkOutcome(testCase, proof, query);
        });
      }
    }
  }

  for (const testCase of cases) {
    for (const proof of [false, true]) {
      const name = proof ? testCase.proofMethod : testCase.method;
      for (const form of ['bytes', 'base58', 'hex', 'array', 'instance'] as IdentifierForm[]) {
        it(`should parse ${name} with ${form} identifiers before its next validation`, async () => {
          await checkOutcome(testCase, proof, testCase.query((byte) => identifier(byte, form)));
        });
      }

      for (const path of testCase.paths) {
        it(`should accept an Identifier at ${name}.${path}`, async () => {
          const query = testCase.query((byte, field) => identifier(byte, field === path ? 'instance' : 'bytes'));
          await checkOutcome(testCase, proof, query);
        });
      }

      it(`should preserve mixed IdentifierLike forms in ${name}`, async () => {
        const forms: IdentifierForm[] = ['instance', 'base58', 'bytes', 'hex'];
        const query = testCase.query((byte, path) => identifier(
          byte,
          forms[testCase.paths.indexOf(path) % forms.length],
        ));
        await checkOutcome(testCase, proof, query);
      });
    }
  }

  for (const proof of [false, true]) {
    it(`should parse a memberDataContracts Identifier before owner validation and proof ${proof}`, async () => {
      await checkOutcome({
        ...cases[5],
        message: 'Invalid identity groups query: fromObject: serde deserialization error: '
          + "string decoding error provided string contained invalid character '-' at byte 3",
      }, proof, {
        identityId: identifier(0x11, 'bytes'),
        memberDataContracts: [identifier(0x44, 'instance')],
        ownerDataContracts: ['not-an-identifier'],
      });
    });
  }

  for (const [name, create] of [
    ['fromBytes', () => sdk.Identifier.fromBytes(new Uint8Array(32).fill(0x11))],
    ['fromHex', () => sdk.Identifier.fromHex('11'.repeat(32))],
    ['fromBase58', () => {
      const value = new sdk.Identifier(new Uint8Array(32).fill(0x11));
      trackIdentifier(value);
      return sdk.Identifier.fromBase58(value.toBase58());
    }],
    ['fromBase64', () => sdk.Identifier.fromBase64('ERERERERERERERERERERERERERERERERERERERERERE=')],
  ] as const) {
    it(`should borrow an Identifier created by ${name} repeatedly`, async () => {
      const value = create();
      trackIdentifier(value);
      const testCase = cases[3];
      const query = testCase.query(() => value);
      await checkOutcome(testCase, false, query);
      await checkOutcome(testCase, true, query);
      expect(value.toBytes()).to.deep.equal(new Uint8Array(32).fill(0x11));
    });
  }

  it('should accept a Rust-returned Identifier without consuming it', async () => {
    const contract = sdk.DataContract.fromJSON(contractFixtureV0 as unknown as sdk.DataContractJSON, true, 1);
    const value = contract.id;
    identifiers.push(value);
    contract.free();
    const testCase = cases[3];
    await checkOutcome(testCase, false, testCase.query(() => value));
    await checkOutcome(testCase, true, testCase.query(() => value));
    expect(value.toBase58()).to.equal(contractFixtureV0.id);
  });

  it('should accept the same module Identifier through distinct SDK client objects', async () => {
    const value = identifier(0x11, 'instance') as sdk.Identifier;
    const testCase = cases[3];
    await checkOutcome(testCase, false, testCase.query(() => value));
    await checkOutcome(testCase, true, testCase.query(() => value), otherClient);
    expect(value.toBytes()).to.deep.equal(new Uint8Array(32).fill(0x11));
  });

  it('should use the local Identifier method for a genuine subclass', async () => {
    const value = new CustomIdentifier(new Uint8Array(32).fill(0x11));
    identifiers.push(value);
    const testCase = cases[3];
    await checkOutcome(testCase, false, testCase.query(() => value));
    await checkOutcome(testCase, true, testCase.query(() => value));
    expect(subclassByteCalls).to.equal(0);
  });

  it('should refuse a freed Identifier before invoking its method', async () => {
    const value = new sdk.Identifier(new Uint8Array(32).fill(0x11));
    value.free();
    const testCase = cases[3];
    for (const method of [testCase.method, testCase.proofMethod]) {
      const error = await rejection(callQuery(testCase.query(() => value), method));
      expect(error.message).to.match(/^Invalid group infos query:/);
    }
  });

  const malformed: Array<[string, () => unknown]> = [
    ['short bytes', () => new Uint8Array(31)],
    ['long bytes', () => new Uint8Array(33)],
    ['invalid string', () => 'not-an-identifier'],
    ['overflow byte', () => [256, ...new Array(31).fill(0)]],
    ['negative byte', () => [-1, ...new Array(31).fill(0)]],
    ['fractional byte', () => [1.5, ...new Array(31).fill(0)]],
    ['numeric string byte', () => ['1', ...new Array(31).fill(0)]],
    ['boolean byte', () => [true, ...new Array(31).fill(0)]],
    ['null', () => null],
  ];

  for (const testCase of cases) {
    for (const [name, create] of malformed) {
      it(`should refuse ${name} as the ${testCase.name} identifier`, async () => {
        const query = testCase.query((byte, path) => (path === testCase.paths[0] ? create() : identifier(byte, 'bytes')));
        freezeContainers(query);
        for (const method of [testCase.method, testCase.proofMethod]) {
          expect((await rejection(callQuery(query, method))).message)
            .to.match(new RegExp(`^Invalid ${testCase.name} query:`));
        }
      });
    }
  }

  it('should refuse a plain marker object without calling its byte method', async () => {
    let calls = 0;
    const value = { __type: 'Identifier', __wbg_ptr: 0 };
    Object.defineProperty(value, 'toBytes', {
      value: () => {
        calls += 1;
        return new Uint8Array(32).fill(0x11);
      },
    });
    const testCase = cases[3];
    const error = await rejection(callQuery(testCase.query(() => value), 'getGroupInfos'));
    expect(error.message).to.match(/^Invalid group infos query:/);
    expect(calls).to.equal(0);
  });

  it('should reject plain Identifier markers before reading internal pointers', async () => {
    let pointerReads = 0;
    const value = { __type: 'Identifier' };
    Object.defineProperty(value, '__wbg_ptr', {
      get: () => {
        pointerReads += 1;
        throw new Error('plain data must not supply a WASM pointer');
      },
    });
    const testCase = cases[3];
    for (const method of [testCase.method, testCase.proofMethod]) {
      const error = await rejection(callQuery(testCase.query(() => value), method));
      expect(error.message).to.match(/^Invalid group infos query:/);
    }
    expect(pointerReads).to.equal(0);
  });

  for (const [name, localConstructor] of [
    ['plain marker', false],
    ['local constructor impostor', true],
  ] as const) {
    it(`should reject a ${name} before reading instance metadata`, async () => {
      let typeReads = 0;
      let pointerReads = 0;
      const value = {};
      if (localConstructor) {
        Object.defineProperty(value, 'constructor', { value: sdk.Identifier });
      }
      Object.defineProperties(value, {
        __type: {
          get: () => {
            typeReads += 1;
            throw new Error('plain data must not supply instance metadata');
          },
        },
        __wbg_ptr: {
          get: () => {
            pointerReads += 1;
            throw new Error('plain data must not supply a WASM pointer');
          },
        },
      });
      const testCase = cases[3];
      for (const method of [testCase.method, testCase.proofMethod]) {
        const error = await rejection(callQuery(testCase.query(() => value), method));
        expect(error.message).to.match(/^Invalid group infos query:/);
      }
      expect(typeReads).to.equal(0);
      expect(pointerReads).to.equal(0);
    });
  }

  for (const [name, constructor] of [
    ['object', { __struct: 'Identifier' }],
    ['string', 'Identifier'],
    ['null', null],
  ] as const) {
    it(`should refuse a JSON Identifier marker with a ${name} constructor`, async () => {
      const value = { __type: 'Identifier', __wbg_ptr: 0, constructor };
      const testCase = cases[3];
      for (const method of [testCase.method, testCase.proofMethod]) {
        const error = await rejection(callQuery(testCase.query(() => value), method));
        expect(error.message).to.match(/^Invalid group infos query:/);
      }
    });
  }

  it('should reject a wrapper with a different module identity before reading instance data', async () => {
    const otherModuleBrand = {};
    const brandDescriptor = Object.getOwnPropertyDescriptor(CustomIdentifier, '__wasm_dpp_module');
    Object.defineProperty(CustomIdentifier, '__wasm_dpp_module', {
      configurable: true,
      get: () => otherModuleBrand,
    });
    const value = new CustomIdentifier(new Uint8Array(32).fill(0x11));
    identifiers.push(value);
    const descriptors = Object.getOwnPropertyDescriptors(value);
    expect(descriptors.__wbg_ptr).to.have.property('value');
    expect(descriptors.__wbg_ptr.configurable).to.equal(true);
    let typeReads = 0;
    let pointerReads = 0;
    Object.defineProperties(value, {
      __type: {
        configurable: true,
        enumerable: false,
        get: () => {
          typeReads += 1;
          throw new Error('a different module must not supply instance metadata');
        },
      },
      __wbg_ptr: {
        configurable: true,
        enumerable: false,
        get: () => {
          pointerReads += 1;
          throw new Error('a different module must not supply a WASM pointer');
        },
      },
    });
    try {
      const testCase = cases[3];
      for (const method of [testCase.method, testCase.proofMethod]) {
        const error = await rejection(callQuery(testCase.query(() => value), method));
        expect(error.message).to.match(/^Invalid group infos query:/);
      }
      expect(typeReads).to.equal(0);
      expect(pointerReads).to.equal(0);
    } finally {
      Object.defineProperty(value, '__wbg_ptr', descriptors.__wbg_ptr);
      if (descriptors.__type) {
        Object.defineProperty(value, '__type', descriptors.__type);
      } else {
        Reflect.deleteProperty(value, '__type');
      }
      if (brandDescriptor) {
        Object.defineProperty(CustomIdentifier, '__wasm_dpp_module', brandDescriptor);
      } else {
        Reflect.deleteProperty(CustomIdentifier, '__wasm_dpp_module');
      }
    }
  });

  it('should refuse an Identifier marker with JSON prototype data', async () => {
    const value = JSON.parse(
      '{"__type":"Identifier","__wbg_ptr":0,'
      + '"__proto__":{"constructor":{"__struct":"Identifier"}}}',
    );
    const testCase = cases[3];
    for (const method of [testCase.method, testCase.proofMethod]) {
      const error = await rejection(callQuery(testCase.query(() => value), method));
      expect(error.message).to.match(/^Invalid group infos query:/);
    }
  });

  it('should borrow a genuine Identifier without invoking its destruction override', async () => {
    let calls = 0;
    const value = identifier(0x11, 'instance') as sdk.Identifier;
    Object.defineProperty(value, '__destroy_into_raw', {
      configurable: true,
      value: () => {
        calls += 1;
        throw new Error('borrowing must not consume the caller wrapper');
      },
    });
    try {
      const testCase = cases[3];
      await checkOutcome(testCase, false, testCase.query(() => value));
      await checkOutcome(testCase, true, testCase.query(() => value));
      expect(calls).to.equal(0);
      expect(value.toBytes()).to.deep.equal(new Uint8Array(32).fill(0x11));
    } finally {
      Reflect.deleteProperty(value, '__destroy_into_raw');
    }
  });

  for (const [name, message, create] of [
    [
      'unsupported unknown value',
      'Invalid group infos query: platform_value_from_object: Error: invalid type: '
        + 'JsValue(Symbol(unsupported)), expected a valid platform value item',
      () => ({ dataContractId: identifier(0x11, 'instance'), limit: 70000, unknown: Symbol('unsupported') }),
    ],
    [
      'iterable query object',
      'Invalid group infos query: platform_value_from_object: Error: invalid type: '
        + 'JsValue(Map), expected a valid platform value item',
      () => new Map([['dataContractId', identifier(0x11, 'bytes')]]),
    ],
    [
      'malformed identifier string',
      'Invalid group infos query: fromObject: serde deserialization error: '
        + "string decoding error provided string contained invalid character '-' at byte 3",
      () => ({ dataContractId: 'not-an-identifier', limit: 70000 }),
    ],
  ] as const) {
    it(`should preserve the complete error and precedence for an ${name}`, async () => {
      const query = create();
      for (const method of [cases[3].method, cases[3].proofMethod]) {
        expect((await rejection(callQuery(query, method))).message).to.equal(message);
      }
    });
  }

  for (const shape of ['inherited', 'nonenumerable'] as const) {
    it(`should keep a ${shape} identifier field absent from the parsed query`, async () => {
      const value = identifier(0x11, 'instance');
      const query = shape === 'inherited'
        ? Object.assign(Object.create({ dataContractId: value }), { limit: 70000 })
        : Object.defineProperty({ limit: 70000 }, 'dataContractId', { value });
      for (const method of [cases[3].method, cases[3].proofMethod]) {
        expect((await rejection(callQuery(query, method))).message)
          .to.match(/^Invalid group infos query:/);
      }
    });
  }

  it('should retain the existing parse error when a repeated identifier getter throws', async () => {
    const value = identifier(0x11, 'instance');
    for (const method of [cases[3].method, cases[3].proofMethod]) {
      let reads = 0;
      const query = Object.defineProperty({ limit: 70000 }, 'dataContractId', {
        enumerable: true,
        get() {
          reads += 1;
          if (reads > 1) {
            throw new Error('identifier getter changed after conversion');
          }
          return value;
        },
      });
      expect((await rejection(callQuery(query, method))).message)
        .to.match(/^Invalid group infos query:/);
      expect(reads).to.be.greaterThan(0);
    }
  });
});
