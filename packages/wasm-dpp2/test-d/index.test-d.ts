// Typings test for the contract configuration surface. `yarn test:types`
// compiles this file against `dist/dpp.d.ts` and never runs it: a line
// that stops compiling, or a `@ts-expect-error` line that starts
// compiling, fails the check.
import type {
  ContractModerationConfig,
  ContractModerators,
  InterimModerators,
  ModerationAbility,
  DataContract,
  DataContractConfig,
  DataContractConfigLike,
  DataContractConfigV0,
  DataContractConfigV1,
  DataContractConfigV2,
} from '@dashevo/wasm-dpp2';
// The canonical corpus rs-dpp generates and pins; its `expect` blocks carry
// every configuration key a mirror must model.
import vectors from '../../rs-dpp/src/data_contract/config/vectors/contract_config_vectors.json' with { type: 'json' };

declare const dataContract: DataContract;

type Equal<A, B> = [A] extends [B] ? ([B] extends [A] ? true : false) : false;
type Check<T extends true> = T;

// The declarations mirror the corpus key sets exactly. `expect` is written in
// JSON form: `sizedIntegerTypes` is absent from the V0 cases and `moderation`
// from every case but the moderated V2 one, so those are the only optional
// corpus keys, and an absent key requirement is `null` where object form
// carries `undefined`. A key added to the corpus without a declaration, or a
// declaration key the corpus never carries, fails here.
type CorpusExpect = (typeof vectors)['cases'][number]['expect'];
type CorpusKeys = Exclude<keyof CorpusExpect, '$formatVersion'>;
// Every key required, `undefined` (the object-form spelling of an absent
// requirement) removed, so the declared value types are compared with the
// JSON-form types the corpus carries, `null` included.
type Shape<T> = { [K in keyof T]-?: Exclude<T[K], undefined> };
type CorpusShape = Shape<Pick<CorpusExpect, CorpusKeys>>;
// The moderation declaration is compared by key set below: each corpus case
// carries one concrete moderators `$type`, the declaration the union of all.
type Flat<T> = Omit<T, '$formatVersion' | 'moderation'>;
type V2Shape = Shape<Flat<DataContractConfigV2>>;
type V1Shape = Shape<Flat<DataContractConfigV1>>;
type V0Shape = Shape<Flat<DataContractConfigV0>>;
type FlagsShape = Shape<Flat<DataContractConfigLike>>;
export type V2MirrorsCorpus = Check<Equal<V2Shape, Omit<CorpusShape, 'moderation'>>>;
export type V1MirrorsCorpus = Check<Equal<V1Shape, Omit<CorpusShape, 'moderation'>>>;
export type V0MirrorsCorpus = Check<Equal<V0Shape, Omit<CorpusShape, 'moderation' | 'sizedIntegerTypes'>>>;
export type SetterInputMirrorsCorpus = Check<Equal<FlagsShape, Omit<CorpusShape, 'moderation'>>>;
export type TagMirrorsCorpus = Check<DataContractConfig['$formatVersion'] extends CorpusExpect['$formatVersion'] ? true : false>;

// `moderation` is declared on V2 and on the setter input, optional on both
// (an unmoderated contract has no such key), on no other generation, and its
// keys and the discriminator of `moderators` mirror the corpus.
type CorpusModeration = NonNullable<CorpusExpect['moderation']>;
export type ModerationOnV2 = Check<Equal<keyof CorpusShape, keyof Shape<Omit<DataContractConfigV2, '$formatVersion'>>>>;
export type ModerationOnSetterInput = Check<Equal<keyof CorpusShape, keyof Shape<Omit<DataContractConfigLike, '$formatVersion'>>>>;
export type ModerationOptionalOnV2 = Check<undefined extends DataContractConfigV2['moderation'] ? true : false>;
export type ModerationOptionalOnSetterInput = Check<undefined extends DataContractConfigLike['moderation'] ? true : false>;
export type ModerationNotOnV1 = Check<'moderation' extends keyof DataContractConfigV1 ? false : true>;
export type ModerationNotOnV0 = Check<'moderation' extends keyof DataContractConfigV0 ? false : true>;
export type ModerationMirrorsCorpus = Check<Equal<keyof ContractModerationConfig, keyof CorpusModeration>>;
export type ModerationListsAreFlags = Check<Equal<
  Pick<ContractModerationConfig, 'banlist' | 'suspensions' | 'warnings'>,
  Pick<CorpusModeration, 'banlist' | 'suspensions' | 'warnings'>
>>;
// The corpus carries the owner kind and the elected kind, and its `$type`
// discriminators are widened to plain strings by the JSON import, so the
// kinds are compared by key set: every key the corpus moderators carry is
// declared, and every declared key but the appointed set's `identities`
// (whose kind the corpus does not carry, an identifier rendering differently
// in object form and in JSON form) is carried by the corpus.
type KeysOfUnion<T> = T extends unknown ? keyof T : never;
export type ModeratorsMirrorCorpus = Check<Equal<Exclude<KeysOfUnion<ContractModerators>, 'identities'>, KeysOfUnion<CorpusModeration['moderators']>>>;
type CorpusElected = Extract<CorpusModeration['moderators'], { interim: unknown }>;
type DeclaredElected = Extract<ContractModerators, { $type: 'elected' }>;
export type ElectedMirrorsCorpus = Check<Equal<keyof Shape<DeclaredElected>, keyof CorpusElected>>;
export type ElectedWindowsAreSeconds = Check<Equal<
  Pick<Shape<DeclaredElected>, 'joinWindow' | 'voteWindow' | 'challengeCoolDown' | 'electionDelay' | 'ownerProtected'>,
  Pick<CorpusElected, 'joinWindow' | 'voteWindow' | 'challengeCoolDown' | 'electionDelay' | 'ownerProtected'>
>>;
export type InterimMirrorsCorpus = Check<Equal<Exclude<KeysOfUnion<InterimModerators>, 'identities'>, keyof CorpusElected['interim']>>;
export type AbilitiesAreNames = Check<CorpusElected['moderatedDocumentTypes'][keyof CorpusElected['moderatedDocumentTypes']][number] extends string ? true : false>;
export type DeclaredAbilitiesAreNames = Check<ModerationAbility extends string ? true : false>;

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

// Narrowing on the tag exposes `sizedIntegerTypes` on format versions 1 and
// 2 only, and `moderation` on format version 2 only.
if (config.$formatVersion === '1') {
  expectAssignable<boolean>(config.sizedIntegerTypes);
  // @ts-expect-error moderation is not a V1 field
  expectAssignable<unknown>(config.moderation);
}
if (config.$formatVersion === '2') {
  expectAssignable<boolean>(config.sizedIntegerTypes);
  expectAssignable<ContractModerationConfig | undefined>(config.moderation);
}
if (config.$formatVersion === '0') {
  // @ts-expect-error sizedIntegerTypes is not a V0 field
  expectAssignable<unknown>(config.sizedIntegerTypes);
}

// A moderation declaration rides along on the setter input, as the bare
// flags or on top of the getter's output.
const moderation: ContractModerationConfig = {
  banlist: true,
  suspensions: false,
  warnings: false,
  moderators: { $type: 'contractOwner' },
};
dataContract.setConfig({ ...flags, moderation }, 14);
dataContract.setConfig({ ...config, moderation }, 14);
dataContract.setConfig({
  ...flags,
  moderation: { ...moderation, moderators: { $type: 'appointedModerators', identities: ['11111111111111111111111111111111'] } },
}, 14);
// @ts-expect-error appointed moderators name their identities
dataContract.setConfig({ ...flags, moderation: { ...moderation, moderators: { $type: 'appointedModerators' } } }, 14);
// @ts-expect-error every list flag is declared
dataContract.setConfig({ ...flags, moderation: { banlist: true, suspensions: false, moderators: { $type: 'contractOwner' } } }, 14);

// An elected declaration: the cool-down, the moderated types and the interim
// moderators are required, the windows, the delay and the flag optional.
const elected: ContractModerators = {
  $type: 'elected',
  challengeCoolDown: 1209600,
  moderatedDocumentTypes: { note: ['ban', 'warn'] },
  interim: { $type: 'notYetUsable' },
};
dataContract.setConfig({ ...flags, moderation: { ...moderation, moderators: elected } }, 14);
dataContract.setConfig({
  ...flags,
  moderation: {
    ...moderation,
    moderators: {
      ...elected, joinWindow: 86400, voteWindow: 172800, electionDelay: 3600, ownerProtected: true,
    },
  },
}, 14);
// @ts-expect-error the cool-down has no default
dataContract.setConfig({ ...flags, moderation: { ...moderation, moderators: { $type: 'elected', moderatedDocumentTypes: {}, interim: { $type: 'noModeration' } } } }, 14);
// @ts-expect-error an ability is one of the four
dataContract.setConfig({ ...flags, moderation: { ...moderation, moderators: { ...elected, moderatedDocumentTypes: { note: ['delete'] } } } }, 14);

// A tag read back from JSON is a plain string; `setConfig` still accepts it.
const fromJson: { $formatVersion: string } & typeof flags = { $formatVersion: '1', ...flags };
dataContract.setConfig(fromJson, 1);

// The flags stay required and typed.
// @ts-expect-error the other five flags are missing
dataContract.setConfig({ canBeDeleted: true }, 1);
// @ts-expect-error a flag must be a boolean
dataContract.setConfig({ ...flags, canBeDeleted: 'yes' }, 1);
