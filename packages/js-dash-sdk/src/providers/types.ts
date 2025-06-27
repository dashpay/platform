/**
 * Types for advanced context providers including web service and priority support
 */

import { ContextProvider } from '../core/types';

export interface QuorumPublicKey {
  version: number;
  publicKey: string;
  type: 'ECDSA' | 'BLS';
}

export interface QuorumInfo {
  quorumHash: string;
  quorumPublicKey: QuorumPublicKey;
  quorumIndex?: number;
  isActive: boolean;
}

export interface QuorumServiceResponse {
  [quorumHash: string]: {
    publicKey: string;
    version: number;
    type: string;
  };
}

export interface WebServiceProviderOptions {
  url?: string;
  network?: 'mainnet' | 'testnet';
  apiKey?: string;
  timeout?: number;
  cacheDuration?: number;
  retryAttempts?: number;
  retryDelay?: number;
}

export interface PriorityProviderOptions {
  providers: Array<{
    provider: ContextProvider;
    priority: number;
    name?: string;
    capabilities?: ProviderCapability[];
  }>;
  fallbackEnabled?: boolean;
  cacheResults?: boolean;
  logErrors?: boolean;
}

export enum ProviderCapability {
  // Context provider capabilities
  PLATFORM_STATE = 'PLATFORM_STATE',
  QUORUM_KEYS = 'QUORUM_KEYS',
  BLOCK_PROPOSER = 'BLOCK_PROPOSER',
  
  // Extended capabilities (for future use)
  SIGNING = 'SIGNING',
  BROADCASTING = 'BROADCASTING',
  SUBSCRIPTIONS = 'SUBSCRIPTIONS',
}

export interface ProviderWithCapabilities extends ContextProvider {
  getCapabilities(): ProviderCapability[];
  getName(): string;
  isAvailable(): Promise<boolean>;
}

export interface ProviderMetrics {
  successCount: number;
  errorCount: number;
  averageResponseTime: number;
  lastError?: Error;
  lastSuccessTime?: Date;
}

export interface PriorityProviderEvents {
  'provider:used': (providerName: string, method: string) => void;
  'provider:error': (providerName: string, error: Error) => void;
  'provider:fallback': (fromProvider: string, toProvider: string) => void;
  'all:failed': (method: string, errors: Map<string, Error>) => void;
}