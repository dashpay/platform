import DAPIClient from '@dashevo/dapi-client';
import { Identifier } from '@dashevo/wasm-dpp/dist';
import { GetDataContractResponse } from '@dashevo/dapi-client/lib/methods/platform/getDataContract/GetDataContractResponse';
import { GetIdentityResponse } from '@dashevo/dapi-client/lib/methods/platform/getIdentity/GetIdentityResponse';

import NotFoundError from '@dashevo/dapi-client/lib/transport/GrpcTransport/errors/NotFoundError';
import { GetDocumentsResponse } from '@dashevo/dapi-client/lib/methods/platform/getDocuments/GetDocumentsResponse';
import {
  GetDataContractHistoryResponse,
} from '@dashevo/dapi-client/lib/methods/platform/getDataContractHistory/GetDataContractHistoryResponse';
import withRetry from './withRetry';
import { QueryOptions } from '../types';

type FetcherOptions = {
  /**
   * Multiplier for delay between retry attempts
   */
  delayMulMs?: number;
  /**
   * Maximum number of retry attempts
   */
  maxAttempts?: number;
};

const DEFAULT_DELAY_MUL_MS = 1000;
const DEFAULT_MAX_ATTEMPTS = 7;

/**
 * Fetcher class that handles retry attempts for acknowledged identifiers
 * Primary goal of this class is to mitigate network propagation lag
 * where we query platform entities right after their creation
 *
 * Should be used until fully functioning state transition acknowledgement is implemented
 *
 * Note: possible collisions of acknowledged keys
 * should be resolved externally by user of this class
 */
class Fetcher {
  public dapiClient: DAPIClient;

  private acknowledgedKeys: Set<string>;

  readonly delayMulMs: number;

  readonly maxAttempts: number;

  constructor(dapiClient: DAPIClient, options: FetcherOptions = {}) {
    this.dapiClient = dapiClient;
    this.acknowledgedKeys = new Set();

    this.delayMulMs = typeof options.delayMulMs === 'number'
      ? options.delayMulMs : DEFAULT_DELAY_MUL_MS;
    this.maxAttempts = typeof options.maxAttempts === 'number'
      ? options.maxAttempts : DEFAULT_MAX_ATTEMPTS;
  }

  /**
   * Acknowledges DPP Identifier to retry on it in get methods
   * @param identifier
   */
  public acknowledgeIdentifier(identifier: Identifier) {
    this.acknowledgedKeys.add(identifier.toString());
  }

  /**
   * Acknowledges string key to retry on it in get methods
   * @param key
   */
  public acknowledgeKey(key: string) {
    this.acknowledgedKeys.add(key);
  }

  /**
   * Forgets string key to stop retrying on it in get methods
   * @param key
   */
  public forgetKey(key: string) {
    this.acknowledgedKeys.delete(key);
  }

  /**
   * Checks if identifier was acknowledged
   * @param identifier
   */
  public hasIdentifier(identifier: Identifier): boolean {
    return this.acknowledgedKeys.has(identifier.toString());
  }

  public hasKey(key: string): boolean {
    return this.acknowledgedKeys.has(key);
  }

  /**
   * Fetches identity by it's ID
   * @param id
   */
  public async fetchIdentity(id: Identifier): Promise<GetIdentityResponse> {
    // Define query
    const query = async (): Promise<GetIdentityResponse> => {
      const { platform } = this.dapiClient;
      return platform.getIdentity(id);
    };

    // Define retry attempts.
    // In case we acknowledged this identifier, we want to retry to mitigate
    // state transition propagation lag. Otherwise, we want to try only once.
    const retryAttempts = this.hasIdentifier(id) ? this.maxAttempts : 1;
    return withRetry(query, retryAttempts, this.delayMulMs);
  }

  /**
   * Fetches data contract by it's ID
   * @param id
   */
  public async fetchDataContract(id: Identifier): Promise<GetDataContractResponse> {
    // Define query
    const query = async (): Promise<GetDataContractResponse> => {
      const { platform } = this.dapiClient;
      return platform.getDataContract(id);
    };

    // Define retry attempts.
    // In case we acknowledged this identifier, we want to retry to mitigate
    // state transition propagation lag. Otherwise, we want to try only once.
    const retryAttempts = this.hasIdentifier(id) ? this.maxAttempts : 1;
    return withRetry(query, retryAttempts, this.delayMulMs);
  }

  /**
   * Fetches data contract by it's ID
   * @param id
   * @param startAMs
   * @param limit
   * @param offset
   */
  public async fetchDataContractHistory(
    id: Identifier,
    startAMs: number,
    limit: number,
    offset: number,
  ): Promise<GetDataContractHistoryResponse> {
    // Define query
    const query = async (): Promise<GetDataContractHistoryResponse> => await this
      .dapiClient.platform.getDataContractHistory(id, startAMs, limit, offset);

    // Define retry attempts.
    // In case we acknowledged this identifier, we want to retry to mitigate
    // state transition propagation lag. Otherwise, we want to try only once.
    const retryAttempts = this.hasIdentifier(id) ? this.maxAttempts : 1;
    return withRetry(query, retryAttempts, this.delayMulMs);
  }

  /**
   * Fetches documents by data contract id and type
   * @param {Identifier} contractId - data contract ID
   * @param {string} type - document name
   * @param {QueryOptions} opts - query
   */
  public async fetchDocuments(
    contractId: Identifier,
    type: string,
    opts: QueryOptions,
  ): Promise<GetDocumentsResponse> {
    // Define query
    const query = async (): Promise<GetDocumentsResponse> => {
      const result = await this.dapiClient.platform
        .getDocuments(contractId, type, opts);

      if (result.getDocuments().length === 0) {
        throw new NotFoundError(`Documents of type "${type}" not found for the data contract ${contractId}`);
      }
      return result;
    };

    // Define retry attempts.
    // In case we acknowledged this identifier, we want to retry to mitigate
    // state transition propagation lag. Otherwise, we want to try only once.
    const documentLocator = `${contractId.toString()}/${type}`;
    const retryAttempts = this.hasKey(documentLocator) ? this.maxAttempts : 1;
    return withRetry(query, retryAttempts, this.delayMulMs);
  }
}

export default Fetcher;
