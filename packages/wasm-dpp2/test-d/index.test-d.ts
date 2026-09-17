// Typings test for the contract configuration surface. `yarn test:types`
// compiles this file against `dist/dpp.d.ts` and never runs it: a line
// that stops compiling, or a `@ts-expect-error` line that starts
// compiling, fails the check.
import type { DataContract, DataContractConfig, DataContractConfigLike } from '@dashevo/wasm-dpp2';

declare const dataContract: DataContract;

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
