import { DataContract, Identifier } from '@dashevo/wasm-dpp';
import {
  GetDataContractHistoryResponse,
} from '@dashevo/dapi-client/lib/methods/platform/getDataContractHistory/GetDataContractHistoryResponse';
import { Platform } from '../../Platform';

const NotFoundError = require('@dashevo/dapi-client/lib/transport/GrpcTransport/errors/NotFoundError');

declare type ContractIdentifier = string | Identifier;

/**
 * Get contracts from the platform
 *
 * @param {ContractIdentifier} identifier - identifier of the contract to fetch
 * @param {bigint} startAtMs
 * @param {number} limit
 * @param {number} offset
 * @returns contracts
 */
export async function history(
  this: Platform,
  identifier: ContractIdentifier,
  startAtMs: bigint,
  limit: number,
  offset: number,
): Promise<any> {
  this.logger.debug(`[Contracts#history] Get Data Contract History for "${identifier}"`);
  await this.initialize();

  const contractId : Identifier = Identifier.from(identifier);

  let dataContractHistoryResponse: GetDataContractHistoryResponse;
  try {
    dataContractHistoryResponse = await this.fetcher
      .fetchDataContractHistory(contractId, startAtMs, limit, offset);
    this.logger.silly(`[Contracts#history] Fetched Data Contract History for "${identifier}"`);
  } catch (e) {
    if (e instanceof NotFoundError) {
      return null;
    }

    throw e;
  }

  const dataContractHistory = dataContractHistoryResponse.getDataContractHistory();
  const contractHistory: { [key: number]: DataContract } = {};

  // eslint-disable-next-line no-restricted-syntax
  for (const dataContractHistoryEntry of dataContractHistory) {
    contractHistory[Number(dataContractHistoryEntry.getDate().toString())] = await this.dpp
      .dataContract.createFromBuffer(dataContractHistoryEntry.getValue() as Uint8Array);
  }

  this.logger.debug(`[Contracts#history] Obtained Data Contract history for "${identifier}"`);

  return contractHistory;
}

export default history;
