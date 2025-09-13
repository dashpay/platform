export type WasmSdkErrorCodes =
  | 'E_INVALID_ARGUMENT'
  | 'E_UNSUPPORTED_VERSION'
  | 'E_NOT_FOUND'
  | 'E_TIMEOUT'
  | 'E_NETWORK'
  | 'E_NETWORK_UNAVAILABLE'
  | 'E_ALREADY_EXISTS'
  | 'E_INTERNAL'
  | 'E_PROTOCOL'
  | 'E_PROOF'
  | 'E_CONTEXT'
  | 'E_CANCELLED'
  | 'E_BROADCAST';

export interface WasmSdkError extends Error {
  code?: WasmSdkErrorCodes;
  kind?: string;
  details?: unknown;
  retriable?: boolean;
}

