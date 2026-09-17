// Typings test for the contract configuration surface. `yarn test:types`
// compiles this file against `dist/dpp.d.ts` and never runs it: a line
// that stops compiling, or a `@ts-expect-error` line that starts
// compiling, fails the check.
import type {
  DataContract, DataContractConfig, DataContractConfigLike, DataContractConfigV0, DataContractConfigV1,
} from '@dashevo/wasm-dpp2';
// The canonical corpus rs-dpp generates and pins; its `expect` blocks carry
// every configuration key a mirror must model.
import vectors from '../../rs-dpp/src/data_contract/config/vectors/contract_config_vectors.json' with { type: 'json' };

declare const dataContract: DataContract;

type Equal<A, B> = [A] extends [B] ? ([B] extends [A] ? true : false) : false;
type Check<T extends true> = T;

// The declarations mirror the corpus key sets exactly. `expect` is written in
// JSON form: `sizedIntegerTypes` is absent from the V0 cases, so it is the
// only optional corpus key, and an absent key requirement is `null` where
// object form carries `undefined`. A key added to the corpus without a
// declaration, or a declaration key the corpus never carries, fails here.
type CorpusExpect = (typeof vectors)['cases'][number]['expect'];
type CorpusKeys = Exclude<keyof CorpusExpect, '$formatVersion'>;
// Every key required, `undefined` (the object-form spelling of an absent
// requirement) removed, so the declared value types are compared with the
// JSON-form types the corpus carries, `null` included.
type Shape<T> = { [K in keyof T]-?: Exclude<T[K], undefined> };
type CorpusShape = Shape<Pick<CorpusExpect, CorpusKeys>>;
type V1Shape = Shape<Omit<DataContractConfigV1, '$formatVersion'>>;
type V0Shape = Shape<Omit<DataContractConfigV0, '$formatVersion'>>;
type FlagsShape = Shape<Omit<DataContractConfigLike, '$formatVersion'>>;
export type V1MirrorsCorpus = Check<Equal<V1Shape, CorpusShape>>;
export type V0MirrorsCorpus = Check<Equal<V0Shape, Omit<CorpusShape, 'sizedIntegerTypes'>>>;
export type SetterInputMirrorsCorpus = Check<Equal<FlagsShape, CorpusShape>>;
export type TagMirrorsCorpus = Check<DataContractConfig['$formatVersion'] extends CorpusExpect['$formatVersion'] ? true : false>;

function expectAssignable<T>(value: T): T {
  return value;
}

// The six flags without a format tag: the shape `setConfig` accepted before
// the configuration type carried `$formatVersion`. It must keep compiling
// without a cast.
const flags = {
  canBeDeleted: false,
  readonly: false,
  keepsHistory: false,
  documentsKeepHistoryContractDefault: false,
  documentsMutableContractDefault: true,
  documentsCanBeDeletedContractDefault: true,
};
dataContract.setConfig(flags, 1);
dataContract.setConfig({ ...flags, requiresIdentityEncryptionBoundedKey: 0 }, 1);
dataContract.setConfig({ ...flags, requiresIdentityDecryptionBoundedKey: null }, 1);
dataContract.setConfig({ ...flags, sizedIntegerTypes: false }, 1);
expectAssignable<DataContractConfigLike>(flags);

// The getter returns the tagged union, and its output feeds `setConfig`
// back without a cast, edited or not.
const config: DataContractConfig = dataContract.config;
dataContract.setConfig(config, 1);
dataContract.setConfig({ ...config, canBeDeleted: !config.canBeDeleted }, 1);

// Narrowing on the tag exposes `sizedIntegerTypes` on format version 1 only.
if (config.$formatVersion === '1') {
  expectAssignable<boolean>(config.sizedIntegerTypes);
}
if (config.$formatVersion === '0') {
  // @ts-expect-error sizedIntegerTypes is not a V0 field
  expectAssignable<unknown>(config.sizedIntegerTypes);
}

// A tag read back from JSON is a plain string; `setConfig` still accepts it.
const fromJson: { $formatVersion: string } & typeof flags = { $formatVersion: '1', ...flags };
dataContract.setConfig(fromJson, 1);

// The flags stay required and typed.
// @ts-expect-error the other five flags are missing
dataContract.setConfig({ canBeDeleted: true }, 1);
// @ts-expect-error a flag must be a boolean
dataContract.setConfig({ ...flags, canBeDeleted: 'yes' }, 1);
